//! Лимиты построений (спека поставки, § 11): слоты реплики с очередью, не больше двух построений
//! на пользователя хоста, `build_timeout` без учёта очереди.

use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use longpole_core::error::{Error, ErrorCode};
use tokio::sync::{Semaphore, SemaphorePermit};

/// Построений одного пользователя хоста одновременно, включая ждущие в очереди.
const PER_USER: usize = 2;

/// Пользователь хоста: домен и id у провайдера.
pub(crate) type User = (String, u64);

pub(crate) struct Limits {
    slots: Semaphore,
    users: Arc<Mutex<HashMap<User, usize>>>,
    timeout: Duration,
}

impl Limits {
    pub(crate) fn new(slots: NonZeroUsize, timeout: Duration) -> Self {
        Self {
            slots: Semaphore::new(slots.get()),
            users: Arc::default(),
            timeout,
        }
    }

    /// Место построения пользователя до конца сборки; `None` — у него уже идут два.
    pub(crate) fn enter(&self, user: User) -> Option<Turn> {
        let mut users = self.users.lock().unwrap_or_else(PoisonError::into_inner);
        let count = users.entry(user.clone()).or_default();
        if *count == PER_USER {
            return None;
        }
        *count += 1;
        Some(Turn {
            users: self.users.clone(),
            user,
        })
    }

    /// Слот реплики; если все заняты — `on_queued`, и ждём в очереди (семафор tokio — по порядку прихода).
    pub(crate) async fn slot(&self, on_queued: impl FnOnce()) -> SemaphorePermit<'_> {
        if let Ok(permit) = self.slots.try_acquire() {
            return permit;
        }
        on_queued();
        self.slots.acquire().await.expect("семафор не закрывается")
    }

    /// `build_timeout`: отмена бросает сборку целиком — в кэш она ничего не кладёт.
    pub(crate) async fn within_timeout<T>(
        &self,
        work: impl Future<Output = Result<T, Error>>,
    ) -> Result<T, Error> {
        tokio::time::timeout(self.timeout, work)
            .await
            .unwrap_or_else(|_| Err(Error::new(ErrorCode::BuildTimeout)))
    }
}

/// Построение пользователя; уходит со сборкой — и когда закрытая вкладка её отменила.
pub(crate) struct Turn {
    users: Arc<Mutex<HashMap<User, usize>>>,
    user: User,
}

impl Drop for Turn {
    fn drop(&mut self) {
        let mut users = self.users.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(count) = users.get_mut(&self.user) {
            *count -= 1;
            if *count == 0 {
                users.remove(&self.user);
            }
        }
    }
}
