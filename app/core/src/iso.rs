use time::format_description::FormatItem;
use time::macros::format_description;
use time::{OffsetDateTime, UtcOffset};

const FORMAT: &[FormatItem<'_>] =
    format_description!("[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z");

/// Как `Date.prototype.toISOString`: UTC, миллисекунды.
pub(crate) fn iso(t: OffsetDateTime) -> String {
    t.to_offset(UtcOffset::UTC)
        .format(FORMAT)
        .expect("формат без недопустимых компонентов")
}
