use anyhow::{Result, anyhow};
use chrono::{DateTime, LocalResult, NaiveDate, NaiveDateTime, TimeZone, Utc};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::models::{Task, TaskPriority};

pub const MAX_ICS_BYTES: usize = 512 * 1024;
pub const MAX_EVENTS: usize = 500;
const MAX_UNFOLDED_LINE_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarEvent {
    pub uid: String,
    pub summary: String,
    pub description: Option<String>,
    pub due_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarImport {
    pub events: Vec<CalendarEvent>,
    pub cancellations: Vec<String>,
}

#[derive(Default)]
struct EventBuilder {
    uid: Option<String>,
    summary: Option<String>,
    description: Option<String>,
    starts_at: Option<DateTime<Utc>>,
    due_at: Option<DateTime<Utc>>,
    cancelled: bool,
}

pub fn parse_ics(input: &str) -> Result<CalendarImport> {
    if input.len() > MAX_ICS_BYTES {
        return Err(anyhow!("Calendar file exceeds the 512 KiB limit"));
    }
    if input.contains('\0') {
        return Err(anyhow!(
            "Calendar file contains unsupported control characters"
        ));
    }

    let mut unfolded = Vec::<String>::new();
    for raw_line in input.replace("\r\n", "\n").replace('\r', "\n").lines() {
        if raw_line.starts_with([' ', '\t']) {
            let previous = unfolded
                .last_mut()
                .ok_or_else(|| anyhow!("Calendar begins with an invalid folded line"))?;
            previous.push_str(&raw_line[1..]);
            if previous.len() > MAX_UNFOLDED_LINE_BYTES {
                return Err(anyhow!("Calendar contains an excessively long line"));
            }
        } else {
            if raw_line.len() > MAX_UNFOLDED_LINE_BYTES {
                return Err(anyhow!("Calendar contains an excessively long line"));
            }
            unfolded.push(raw_line.to_string());
        }
    }

    let mut events = Vec::new();
    let mut cancellations = Vec::new();
    let mut seen_uids = std::collections::HashSet::new();
    let mut current: Option<EventBuilder> = None;
    for line in unfolded {
        if line.eq_ignore_ascii_case("BEGIN:VEVENT") {
            if current.is_some() {
                return Err(anyhow!("Calendar contains nested VEVENT blocks"));
            }
            current = Some(EventBuilder::default());
            continue;
        }
        if line.eq_ignore_ascii_case("END:VEVENT") {
            let builder = current
                .take()
                .ok_or_else(|| anyhow!("Calendar closes a VEVENT that was not opened"))?;
            let number = events.len() + cancellations.len() + 1;
            if number > MAX_EVENTS {
                return Err(anyhow!("Calendar contains more than {MAX_EVENTS} events"));
            }
            let uid = required_text(builder.uid, "UID", number, 512, false)?;
            if !seen_uids.insert(uid.clone()) {
                return Err(anyhow!("Calendar contains duplicate event UID '{uid}'"));
            }
            if builder.cancelled {
                cancellations.push(uid);
            } else {
                let summary = required_text(builder.summary, "SUMMARY", number, 120, false)?;
                let description = builder
                    .description
                    .map(|value| validate_text(value, "DESCRIPTION", number, 2_000, true))
                    .transpose()?;
                let due_at = builder
                    .due_at
                    .or(builder.starts_at)
                    .ok_or_else(|| anyhow!("Calendar event {number} has no DTSTART or DUE"))?;
                events.push(CalendarEvent {
                    uid,
                    summary,
                    description,
                    due_at,
                });
            }
            continue;
        }

        let Some(builder) = current.as_mut() else {
            continue;
        };
        let Some((raw_name, raw_value)) = line.split_once(':') else {
            continue;
        };
        let name = raw_name
            .split(';')
            .next()
            .unwrap_or(raw_name)
            .to_ascii_uppercase();
        match name.as_str() {
            "UID" => builder.uid = Some(unescape_text(raw_value)),
            "SUMMARY" => builder.summary = Some(unescape_text(raw_value)),
            "DESCRIPTION" => builder.description = Some(unescape_text(raw_value)),
            "DTSTART" => {
                builder.starts_at = Some(parse_ical_datetime(raw_value, property_tzid(raw_name))?)
            }
            "DUE" => {
                builder.due_at = Some(parse_ical_datetime(raw_value, property_tzid(raw_name))?)
            }
            "STATUS" if raw_value.eq_ignore_ascii_case("CANCELLED") => builder.cancelled = true,
            _ => {}
        }
    }
    if current.is_some() {
        return Err(anyhow!("Calendar contains an unclosed VEVENT"));
    }
    if events.is_empty() && cancellations.is_empty() {
        return Err(anyhow!("Calendar contains no importable events"));
    }
    Ok(CalendarImport {
        events,
        cancellations,
    })
}

pub fn task_id_for_event(project_id: Uuid, uid: &str) -> Uuid {
    let mut hasher = Sha256::new();
    hasher.update(b"questline-calendar-event-v1");
    hasher.update(project_id.as_bytes());
    hasher.update(uid.as_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    // RFC 4122 variant with a stable, name-derived version marker.
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

pub fn materialize_events(
    project_id: Uuid,
    events: &[CalendarEvent],
    owner_identity: &str,
    owner_username: &str,
) -> Vec<Task> {
    let now = Utc::now();
    events
        .iter()
        .map(|event| Task {
            id: task_id_for_event(project_id, &event.uid),
            project_id: Some(project_id),
            title: event.summary.clone(),
            description: event.description.clone(),
            due_date: Some(event.due_at),
            set_date: None,
            completed: false,
            priority: TaskPriority::Medium,
            created_at: now,
            updated_at: now,
            owner_identity: Some(owner_identity.to_string()),
            owner_username: Some(owner_username.to_string()),
            parent_task_id: None,
            xp_awarded: false,
            recurrence: None,
        })
        .collect()
}

fn required_text(
    value: Option<String>,
    field: &str,
    event_number: usize,
    max_chars: usize,
    allow_layout_controls: bool,
) -> Result<String> {
    let value = value.ok_or_else(|| anyhow!("Calendar event {event_number} has no {field}"))?;
    validate_text(value, field, event_number, max_chars, allow_layout_controls)
}

fn validate_text(
    value: String,
    field: &str,
    event_number: usize,
    max_chars: usize,
    allow_layout_controls: bool,
) -> Result<String> {
    if value.trim().is_empty() {
        return Err(anyhow!(
            "Calendar event {event_number} has an empty {field}"
        ));
    }
    if value.chars().count() > max_chars {
        return Err(anyhow!(
            "Calendar event {event_number} {field} exceeds {max_chars} characters"
        ));
    }
    if value.chars().any(|character| {
        character.is_control() && !(allow_layout_controls && matches!(character, '\n' | '\t'))
    }) {
        return Err(anyhow!(
            "Calendar event {event_number} {field} contains unsupported control characters"
        ));
    }
    Ok(value)
}

fn unescape_text(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(character) = chars.next() {
        if character != '\\' {
            output.push(character);
            continue;
        }
        match chars.next() {
            Some('n' | 'N') => output.push('\n'),
            Some(',') => output.push(','),
            Some(';') => output.push(';'),
            Some('\\') => output.push('\\'),
            Some(other) => {
                output.push('\\');
                output.push(other);
            }
            None => output.push('\\'),
        }
    }
    output
}

fn property_tzid(raw_name: &str) -> Option<&str> {
    raw_name.split(';').skip(1).find_map(|parameter| {
        let (name, value) = parameter.split_once('=')?;
        name.eq_ignore_ascii_case("TZID")
            .then_some(value.trim_matches('"'))
    })
}

fn parse_ical_datetime(value: &str, tzid: Option<&str>) -> Result<DateTime<Utc>> {
    if let Ok(date) = NaiveDate::parse_from_str(value, "%Y%m%d") {
        return Ok(Utc.from_utc_datetime(
            &date
                .and_hms_opt(23, 59, 59)
                .ok_or_else(|| anyhow!("Invalid calendar date"))?,
        ));
    }
    let (value, explicitly_utc) = value
        .strip_suffix('Z')
        .map(|value| (value, true))
        .unwrap_or((value, false));
    let parsed = NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%S")
        .or_else(|_| NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M"))
        .map_err(|_| anyhow!("Unsupported calendar date/time: {value}"))?;
    if explicitly_utc {
        return Ok(Utc.from_utc_datetime(&parsed));
    }
    if let Some(tzid) = tzid {
        let timezone = tzid
            .parse::<chrono_tz::Tz>()
            .map_err(|_| anyhow!("Unsupported calendar TZID: {tzid}"))?;
        return match timezone.from_local_datetime(&parsed) {
            LocalResult::Single(value) => Ok(value.with_timezone(&Utc)),
            LocalResult::Ambiguous(_, _) => Err(anyhow!(
                "Calendar time {value} is ambiguous in timezone {tzid}"
            )),
            LocalResult::None => Err(anyhow!(
                "Calendar time {value} does not exist in timezone {tzid}"
            )),
        };
    }
    Ok(Utc.from_utc_datetime(&parsed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_folded_escaped_and_cancelled_calendar_events() {
        let import = parse_ics(
            "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:event-1\r\nSUMMARY:Release\\, review\r\nDESCRIPTION:First line\\nsecond \r\n line\r\nDTSTART:20260804T150000Z\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:cancelled\r\nSUMMARY:Ignore me\r\nDTSTART;VALUE=DATE:20260805\r\nSTATUS:CANCELLED\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n",
        )
        .unwrap();
        assert_eq!(import.events.len(), 1);
        assert_eq!(import.cancellations, vec!["cancelled"]);
        assert_eq!(import.events[0].summary, "Release, review");
        assert_eq!(
            import.events[0].description.as_deref(),
            Some("First line\nsecond line")
        );
        assert_eq!(
            import.events[0].due_at.to_rfc3339(),
            "2026-08-04T15:00:00+00:00"
        );
    }

    #[test]
    fn rejects_missing_identity_malformed_blocks_and_limits() {
        assert!(parse_ics(&" ".repeat(MAX_ICS_BYTES + 1)).is_err());
        assert!(parse_ics("BEGIN:VEVENT\nSUMMARY:No UID\nDTSTART:20260804\nEND:VEVENT").is_err());
        assert!(parse_ics("BEGIN:VEVENT\nBEGIN:VEVENT\nEND:VEVENT\nEND:VEVENT").is_err());
        assert!(
            parse_ics("BEGIN:VEVENT\nUID:x\nSUMMARY:No closing block\nDTSTART:20260804").is_err()
        );
        assert!(
            parse_ics(
                "BEGIN:VEVENT\nUID:x\nSUMMARY:Forged\\npreview line\nDTSTART:20260804\nEND:VEVENT"
            )
            .is_err()
        );

        let cancellation = "BEGIN:VEVENT\nUID:cancel-{index}\nSTATUS:CANCELLED\nEND:VEVENT\n";
        let oversized_cancellations = (0..=MAX_EVENTS)
            .map(|index| cancellation.replace("{index}", &index.to_string()))
            .collect::<String>();
        assert!(parse_ics(&oversized_cancellations).is_err());
    }

    #[test]
    fn resolves_iana_tzid_and_rejects_unknown_or_nonexistent_times() {
        let import = parse_ics(
            "BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:tz-1\nSUMMARY:Mexico planning\nDTSTART;TZID=America/Mexico_City:20260804T090000\nEND:VEVENT\nEND:VCALENDAR",
        )
        .unwrap();
        assert_eq!(
            import.events[0].due_at.to_rfc3339(),
            "2026-08-04T15:00:00+00:00"
        );
        assert!(
            parse_ics("BEGIN:VEVENT\nUID:tz-2\nSUMMARY:Unknown zone\nDTSTART;TZID=Mars/Olympus:20260804T090000\nEND:VEVENT")
                .is_err()
        );
        assert!(
            parse_ics("BEGIN:VEVENT\nUID:tz-3\nSUMMARY:Skipped hour\nDTSTART;TZID=America/New_York:20260308T023000\nEND:VEVENT")
                .is_err()
        );
    }

    #[test]
    fn event_ids_are_stable_per_campaign_and_materialized_state_is_private() {
        let project = Uuid::new_v4();
        let other_project = Uuid::new_v4();
        assert_eq!(
            task_id_for_event(project, "event-1"),
            task_id_for_event(project, "event-1")
        );
        assert_ne!(
            task_id_for_event(project, "event-1"),
            task_id_for_event(other_project, "event-1")
        );
        let events = vec![CalendarEvent {
            uid: "event-1".to_string(),
            summary: "Planning call".to_string(),
            description: None,
            due_at: Utc::now(),
        }];
        let tasks = materialize_events(project, &events, "owner", "Hero");
        assert_eq!(tasks.len(), 1);
        assert!(!tasks[0].completed);
        assert!(tasks[0].parent_task_id.is_none());
        assert!(tasks[0].recurrence.is_none());
    }
}
