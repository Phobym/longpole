//! Кэш отчётов (спека поставки, § 7): JSON отчёта по завершённому пайплайну, общий для инстанса, в памяти
//! процесса. Права здесь не проверяются — это делает вызывающий перед каждой выдачей.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use serde_json::Value;

/// Пайплайн по числовым id: путь проекта и номер MR разрешены до ключа.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Key {
    pub host: String,
    pub project: String,
    pub pipeline: String,
}

/// LRU по суммарному размеру JSON, без TTL; одновременная сборка одного ключа — одна.
pub(crate) struct ReportCache {
    max_bytes: usize,
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    entries: HashMap<Key, Entry>,
    bytes: usize,
    /// счётчик обращений: у давно не читанной записи он меньше
    clock: u64,
    flights: HashMap<Key, Arc<tokio::sync::Mutex<()>>>,
}

struct Entry {
    version: String,
    report: Arc<Value>,
    size: usize,
    used: u64,
}

impl ReportCache {
    pub(crate) fn new(max_bytes: usize) -> Self {
        Self {
            max_bytes,
            inner: Mutex::default(),
        }
    }

    fn inner(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Отчёт той же версии пайплайна; другая версия — промах.
    pub(crate) fn get(&self, key: &Key, version: &str) -> Option<Arc<Value>> {
        let inner = &mut *self.inner();
        inner.clock += 1;
        let entry = inner
            .entries
            .get_mut(key)
            .filter(|e| e.version == version)?;
        entry.used = inner.clock;
        Some(entry.report.clone())
    }

    /// Запись вытесняет давно не читанные, пока не влезет; больше всего лимита — не кэшируется.
    // ponytail: поиск вытесняемой — проход по всем записям; их сотни, не миллионы
    pub(crate) fn put(&self, key: Key, version: String, report: Value) {
        let size = report.to_string().len();
        if size > self.max_bytes {
            return;
        }
        let inner = &mut *self.inner();
        if let Some(old) = inner.entries.remove(&key) {
            inner.bytes -= old.size;
        }
        while inner.bytes + size > self.max_bytes {
            let Some(oldest) = inner
                .entries
                .iter()
                .min_by_key(|(_, e)| e.used)
                .map(|(k, _)| k.clone())
            else {
                break;
            };
            let gone = inner
                .entries
                .remove(&oldest)
                .expect("ключ только что найден");
            inner.bytes -= gone.size;
        }
        inner.clock += 1;
        inner.bytes += size;
        let used = inner.clock;
        inner.entries.insert(
            key,
            Entry {
                version,
                report: Arc::new(report),
                size,
                used,
            },
        );
    }

    /// `work` по ключу выполняется по одному: второй запрос ждёт первый и затем видит его запись в кэше.
    // ponytail: ждущий незавершённого пайплайна строит заново — такие не кэшируются; общий результат, если понадобится
    pub(crate) async fn single_flight<T>(&self, key: &Key, work: impl Future<Output = T>) -> T {
        let lock = self.inner().flights.entry(key.clone()).or_default().clone();
        let result = {
            let _turn = lock.lock().await;
            work.await
        };
        let mut inner = self.inner();
        // в карте и у нас — больше ждущих нет
        if Arc::strong_count(&lock) == 2 {
            inner.flights.remove(key);
        }
        result
    }
}

/// Пайплайн и все downstream в терминальном статусе: только такой отчёт кэшируется.
pub(crate) fn finished(report: &Value) -> bool {
    let Some(nodes) = report["tree"]["nodes"].as_object() else {
        return false;
    };
    nodes.values().filter(|n| n["kind"] == "pipeline").all(|n| {
        matches!(
            n["status"].as_str(),
            Some("success" | "failed" | "canceled" | "skipped")
        )
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn key(pipeline: &str) -> Key {
        Key {
            host: "h".into(),
            project: "1".into(),
            pipeline: pipeline.into(),
        }
    }

    /// JSON ровно в `size` байт.
    fn report(size: usize) -> Value {
        json!("x".repeat(size - 2))
    }

    #[test]
    fn вытесняется_давно_не_читанный_пока_не_влезет_новый() {
        let cache = ReportCache::new(300);
        cache.put(key("1"), "v".into(), report(100));
        cache.put(key("2"), "v".into(), report(100));
        cache.put(key("3"), "v".into(), report(100));
        assert!(cache.get(&key("1"), "v").is_some());

        cache.put(key("4"), "v".into(), report(150));

        assert!(cache.get(&key("1"), "v").is_some(), "читали недавно");
        assert!(cache.get(&key("2"), "v").is_none());
        assert!(cache.get(&key("3"), "v").is_none());
        assert!(cache.get(&key("4"), "v").is_some());
    }

    #[test]
    fn отчёт_больше_лимита_не_кэшируется_и_не_вытесняет() {
        let cache = ReportCache::new(300);
        cache.put(key("1"), "v".into(), report(100));
        cache.put(key("2"), "v".into(), report(301));
        assert!(cache.get(&key("1"), "v").is_some());
        assert!(cache.get(&key("2"), "v").is_none());
    }

    #[test]
    fn другая_версия_это_промах() {
        let cache = ReportCache::new(300);
        cache.put(key("1"), "v1".into(), report(100));
        assert!(cache.get(&key("1"), "v2").is_none());
    }

    #[test]
    fn завершён_только_если_завершены_все_пайплайны() {
        let tree = |downstream: &str| {
            json!({"tree": {"nodes": {
                "p": {"kind": "pipeline", "status": "success"},
                "j": {"kind": "job", "status": "running"},
                "d": {"kind": "pipeline", "status": downstream},
            }}})
        };
        assert!(finished(&tree("failed")));
        assert!(!finished(&tree("running")));
        assert!(!finished(&tree("manual")));
        assert!(!finished(&json!({"mode": "aggregate"})));
    }
}
