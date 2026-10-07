//! Файл workflow GitHub Actions: `needs`, уровни джоб и сопоставление имён джоб API с джобами YAML
//! (спека, § 2.4–2.5).

use std::collections::{BTreeMap, HashMap};
use std::sync::LazyLock;

use regex::Regex;
use serde::Deserialize;
use serde_yaml_ng::Value;

static EXPR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\$\{\{.*?\}\}").expect("верный шаблон"));

#[derive(Deserialize)]
struct File {
    jobs: BTreeMap<String, YamlJob>,
}

/// Поля джобы, которые нужны отчёту; остальные игнорируются. `Value` — чтобы чужая форма поля не роняла разбор.
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
struct YamlJob {
    name: Option<String>,
    needs: Option<Value>,
    strategy: Option<Value>,
    continue_on_error: Option<Value>,
}

/// Джоба YAML с посчитанным уровнем.
#[derive(Debug)]
pub(crate) struct Job {
    pub(crate) key: String,
    /// `name:` или ключ — так GitHub называет джобу без matrix
    title: String,
    /// `name:` с `${{ }}` и литеральным текстом: шаблон имени из API
    pattern: Option<Regex>,
    pub(crate) needs: Vec<String>,
    matrix: bool,
    pub(crate) allow_failure: bool,
    /// длина самого длинного пути по `needs`
    pub(crate) level: usize,
}

impl Job {
    /// Имя в заголовке стейджа: шаблон показать нельзя — тогда ключ.
    fn display(&self) -> &str {
        if self.pattern.is_some() || EXPR.is_match(&self.title) {
            &self.key
        } else {
            &self.title
        }
    }
}

#[derive(Debug)]
pub(crate) struct Workflow {
    /// по ключу
    jobs: Vec<Job>,
}

/// Джоба API, сопоставленная с джобой YAML.
#[derive(Debug)]
pub(crate) struct Matched<'a> {
    pub(crate) job: &'a Job,
    /// имя для отчёта: шарды matrix — `X: [..]`, их сворачивает `model::shard_group_name`
    pub(crate) name: String,
}

pub(crate) fn parse(text: &str) -> Option<Workflow> {
    let file: File = serde_yaml_ng::from_str(text).ok()?;
    let mut jobs: Vec<Job> = file
        .jobs
        .into_iter()
        .map(|(key, job)| {
            let title = job.name.unwrap_or_else(|| key.clone());
            Job {
                pattern: pattern(&title),
                title,
                needs: needs_list(job.needs.as_ref()),
                matrix: job
                    .strategy
                    .as_ref()
                    .is_some_and(|s| s.get("matrix").is_some()),
                allow_failure: job.continue_on_error == Some(Value::Bool(true)),
                level: 0,
                key,
            }
        })
        .collect();
    let levels = levels(&jobs);
    for (job, level) in jobs.iter_mut().zip(levels) {
        job.level = level;
    }
    Some(Workflow { jobs })
}

fn needs_list(needs: Option<&Value>) -> Vec<String> {
    match needs {
        Some(Value::String(one)) => vec![one.clone()],
        Some(Value::Sequence(many)) => many
            .iter()
            .filter_map(|n| n.as_str().map(String::from))
            .collect(),
        _ => Vec::new(),
    }
}

/// `name:` с выражениями → регулярное выражение: литералы экранированы, выражение — `.*?`.
/// Имя из одних выражений шаблоном не становится: оно совпало бы с любой джобой.
fn pattern(name: &str) -> Option<Regex> {
    if !EXPR.is_match(name) || EXPR.replace_all(name, "").trim().is_empty() {
        return None;
    }
    let mut re = String::from("^");
    let mut last = 0;
    for m in EXPR.find_iter(name) {
        re.push_str(&regex::escape(&name[last..m.start()]));
        re.push_str(".*?");
        last = m.end();
    }
    re.push_str(&regex::escape(&name[last..]));
    re.push('$');
    Regex::new(&re).ok()
}

/// Уровень каждой джобы; рёбра в цикл и к несуществующим джобам не считаются.
fn levels(jobs: &[Job]) -> Vec<usize> {
    let index: HashMap<&str, usize> = jobs
        .iter()
        .enumerate()
        .map(|(i, j)| (j.key.as_str(), i))
        .collect();
    let mut memo = vec![None; jobs.len()];
    let mut visiting = vec![false; jobs.len()];
    (0..jobs.len())
        .map(|i| visit(i, jobs, &index, &mut memo, &mut visiting))
        .collect()
}

fn visit(
    i: usize,
    jobs: &[Job],
    index: &HashMap<&str, usize>,
    memo: &mut [Option<usize>],
    visiting: &mut [bool],
) -> usize {
    if let Some(level) = memo[i] {
        return level;
    }
    visiting[i] = true;
    let mut level = 0;
    for need in &jobs[i].needs {
        if let Some(&n) = index.get(need.as_str())
            && !visiting[n]
        {
            level = level.max(visit(n, jobs, index, memo, visiting) + 1);
        }
    }
    visiting[i] = false;
    memo[i] = Some(level);
    level
}

/// `X (a, b)` → (`X`, `a, b`): так GitHub называет matrix-джобу без выражений в имени.
fn matrix_suffix(name: &str) -> Option<(&str, &str)> {
    name.strip_suffix(')')?.rsplit_once(" (")
}

impl Workflow {
    /// Джоба YAML для имени из API (спека, § 2.5); `caller / inner` — джоба reusable workflow.
    pub(crate) fn find(&self, api_name: &str) -> Option<Matched<'_>> {
        self.find_direct(api_name).or_else(|| {
            let (caller, _) = api_name.split_once(" / ")?;
            let found = self.find_direct(caller)?;
            Some(Matched {
                job: found.job,
                name: api_name.into(),
            })
        })
    }

    fn find_direct(&self, api_name: &str) -> Option<Matched<'_>> {
        let plain = |j: &&Job| j.pattern.is_none();
        if let Some(job) = self.jobs.iter().filter(plain).find(|j| j.title == api_name) {
            return Some(Matched {
                job,
                name: api_name.into(),
            });
        }
        if let Some((base, values)) = matrix_suffix(api_name)
            && let Some(job) = self
                .jobs
                .iter()
                .filter(plain)
                .find(|j| j.matrix && j.title == base)
        {
            return Some(Matched {
                job,
                name: format!("{base}: [{values}]"),
            });
        }
        let job = self
            .jobs
            .iter()
            .find(|j| j.pattern.as_ref().is_some_and(|p| p.is_match(api_name)))?;
        let name = if job.matrix {
            format!("{}: [{api_name}]", job.key)
        } else {
            api_name.into()
        };
        Some(Matched { job, name })
    }

    /// Имена уровней `0..=max` (спека, § 2.4): `a`, `a · b`, `a · b · +N`.
    pub(crate) fn stage_names(&self) -> Vec<String> {
        let max = self.jobs.iter().map(|j| j.level).max().unwrap_or(0);
        (0..=max)
            .map(|level| {
                let names: Vec<&str> = self
                    .jobs
                    .iter()
                    .filter(|j| j.level == level)
                    .map(Job::display)
                    .collect();
                match names.as_slice() {
                    [] => String::new(),
                    [one] => (*one).to_string(),
                    [a, b] => format!("{a} · {b}"),
                    [a, b, rest @ ..] => format!("{a} · {b} · +{}", rest.len()),
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const YAML: &str = r#"
on: push
jobs:
  build:
    runs-on: ubuntu-latest
  lint:
    name: Lint code
    continue-on-error: true
  test:
    needs: build
    strategy:
      matrix:
        os: [linux, mac]
  e2e:
    name: 'e2e ${{ matrix.shard }} on linux'
    needs: [build]
    strategy:
      matrix:
        shard: [1, 2]
  call:
    needs: [test, e2e, lint]
    uses: ./.github/workflows/deploy.yml
  dynamic:
    name: '${{ inputs.title }}'
    continue-on-error: ${{ inputs.soft }}
"#;

    fn key_and_name(w: &Workflow, api: &str) -> Option<(String, String)> {
        w.find(api).map(|m| (m.job.key.clone(), m.name))
    }

    #[test]
    fn уровни_по_needs_строкой_и_списком() {
        let w = parse(YAML).unwrap();
        let level = |key: &str| w.jobs.iter().find(|j| j.key == key).unwrap().level;
        assert_eq!(
            [
                level("build"),
                level("lint"),
                level("dynamic"),
                level("test"),
                level("e2e"),
                level("call")
            ],
            [0, 0, 0, 1, 1, 2]
        );
    }

    #[test]
    fn имена_стейджей_из_джоб_уровня_по_ключу() {
        let w = parse(YAML).unwrap();
        assert_eq!(
            w.stage_names(),
            vec![
                "build · dynamic · +1".to_string(),
                "e2e · test".into(),
                "call".into()
            ]
        );
    }

    #[test]
    fn сопоставление_имён_api() {
        let w = parse(YAML).unwrap();
        assert_eq!(
            key_and_name(&w, "build"),
            Some(("build".into(), "build".into()))
        );
        assert_eq!(
            key_and_name(&w, "Lint code"),
            Some(("lint".into(), "Lint code".into()))
        );
        assert_eq!(
            key_and_name(&w, "test (linux)"),
            Some(("test".into(), "test: [linux]".into()))
        );
        assert_eq!(
            key_and_name(&w, "e2e 1 on linux"),
            Some(("e2e".into(), "e2e: [e2e 1 on linux]".into()))
        );
        assert_eq!(
            key_and_name(&w, "call / deploy prod"),
            Some(("call".into(), "call / deploy prod".into()))
        );
        // шаблон из одних выражений ничего не ловит, иначе забрал бы все имена
        assert_eq!(key_and_name(&w, "что-то своё"), None);
    }

    #[test]
    fn continue_on_error_только_буквальное_true() {
        let w = parse(YAML).unwrap();
        let soft = |key: &str| w.jobs.iter().find(|j| j.key == key).unwrap().allow_failure;
        assert!(soft("lint"));
        assert!(!soft("dynamic"));
        assert!(!soft("build"));
    }

    #[test]
    fn цикл_и_несуществующая_джоба_не_роняют_разбор() {
        let w = parse("jobs:\n  a:\n    needs: [b, ghost]\n  b:\n    needs: a\n").unwrap();
        let level = |key: &str| w.jobs.iter().find(|j| j.key == key).unwrap().level;
        assert_eq!((level("a"), level("b")), (1, 0));
    }

    #[test]
    fn не_yaml_и_файл_без_jobs_дают_none() {
        assert!(parse("jobs: [не закрыто").is_none());
        assert!(parse("on: push\n").is_none());
    }
}
