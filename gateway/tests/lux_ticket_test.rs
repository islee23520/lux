#[path = "../src/lux_ticket.rs"]
mod lux_ticket;

use std::{fs, path::PathBuf};

use lux_ticket::{
    FileTicketStore, Ticket, TicketFilter, TicketPriority, TicketStatus, TicketStore,
};

fn temp_project_root() -> PathBuf {
    let root = std::env::temp_dir().join(format!("lux-ticket-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&root).unwrap();
    root
}

fn ticket(
    id: &str,
    status: TicketStatus,
    blockers: Vec<String>,
    tags: Vec<String>,
    spec_ref: Option<&str>,
) -> Ticket {
    Ticket {
        id: id.to_string(),
        title: "Feature A".to_string(),
        description: "Do the thing".to_string(),
        status,
        priority: TicketPriority::High,
        assignee: Some("dev".to_string()),
        blockers,
        tags,
        spec_ref: spec_ref.map(|value| value.to_string()),
        created_at: "2026-05-11T00:00:00Z".to_string(),
        updated_at: "2026-05-11T00:00:00Z".to_string(),
        ..Default::default()
    }
}

#[test]
fn test_lux_ticket_create_valid() {
    let root = temp_project_root();
    let store = FileTicketStore::new(&root);
    let id = uuid::Uuid::new_v4().to_string();

    let created = store
        .create(ticket(
            &id,
            TicketStatus::Backlog,
            vec![],
            vec!["ui".to_string()],
            Some(".lux/spec/foo"),
        ))
        .unwrap();

    assert_eq!(created.id, id);
    assert!(root
        .join(".lux/tickets")
        .join(format!("{}.json", id))
        .exists());
}

#[test]
fn test_lux_ticket_status_transition_backlog_to_todo() {
    let root = temp_project_root();
    let store = FileTicketStore::new(&root);
    let id = uuid::Uuid::new_v4().to_string();

    store
        .create(ticket(&id, TicketStatus::Backlog, vec![], vec![], None))
        .unwrap();
    let updated = store
        .update(&id, ticket(&id, TicketStatus::ToDo, vec![], vec![], None))
        .unwrap();

    assert_eq!(updated.status, TicketStatus::ToDo);
}

#[test]
fn test_lux_ticket_status_transition_blocked_denied() {
    let root = temp_project_root();
    let store = FileTicketStore::new(&root);
    let blocker_id = uuid::Uuid::new_v4().to_string();
    let ticket_id = uuid::Uuid::new_v4().to_string();

    store
        .create(ticket(
            &blocker_id,
            TicketStatus::InProgress,
            vec![],
            vec![],
            None,
        ))
        .unwrap();
    store
        .create(ticket(
            &ticket_id,
            TicketStatus::Blocked,
            vec![blocker_id],
            vec![],
            None,
        ))
        .unwrap();
    let err = store
        .update(
            &ticket_id,
            ticket(
                &ticket_id,
                TicketStatus::ToDo,
                vec![uuid::Uuid::new_v4().to_string()],
                vec![],
                None,
            ),
        )
        .unwrap_err();

    assert!(err.to_string().contains("transition denied"));
}

#[test]
fn test_lux_ticket_blocker_prevents_progress() {
    let root = temp_project_root();
    let store = FileTicketStore::new(&root);
    let blocker_id = uuid::Uuid::new_v4().to_string();
    let ticket_id = uuid::Uuid::new_v4().to_string();

    store
        .create(ticket(
            &blocker_id,
            TicketStatus::InProgress,
            vec![],
            vec![],
            None,
        ))
        .unwrap();
    store
        .create(ticket(
            &ticket_id,
            TicketStatus::ToDo,
            vec![blocker_id.clone()],
            vec![],
            None,
        ))
        .unwrap();

    let err = store
        .update(
            &ticket_id,
            ticket(
                &ticket_id,
                TicketStatus::InProgress,
                vec![blocker_id],
                vec![],
                None,
            ),
        )
        .unwrap_err();
    assert!(err.to_string().contains("active blockers"));
}

#[test]
fn test_lux_ticket_filter_by_status() {
    let root = temp_project_root();
    let store = FileTicketStore::new(&root);
    let a = uuid::Uuid::new_v4().to_string();
    let b = uuid::Uuid::new_v4().to_string();

    store
        .create(ticket(
            &a,
            TicketStatus::ToDo,
            vec![],
            vec!["alpha".to_string()],
            Some(".lux/spec/a"),
        ))
        .unwrap();
    store
        .create(ticket(
            &b,
            TicketStatus::Done,
            vec![],
            vec!["beta".to_string()],
            Some(".lux/spec/b"),
        ))
        .unwrap();

    let filtered = store
        .list(TicketFilter {
            status: Some(TicketStatus::ToDo),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, a);
}

#[test]
fn test_lux_ticket_file_store_crud() {
    let root = temp_project_root();
    let store = FileTicketStore::new(&root);
    let id = uuid::Uuid::new_v4().to_string();
    let blocker_id = uuid::Uuid::new_v4().to_string();

    store
        .create(ticket(
            &id,
            TicketStatus::Backlog,
            vec![],
            vec!["tag".to_string()],
            Some(".lux/spec/a"),
        ))
        .unwrap();
    let fetched = store.get(&id).unwrap().unwrap();
    assert_eq!(fetched.id, id);

    store
        .create(ticket(
            &blocker_id,
            TicketStatus::Done,
            vec![],
            vec![],
            None,
        ))
        .unwrap();
    let updated_ticket = Ticket {
        id: id.clone(),
        title: "Updated".to_string(),
        description: "Updated description".to_string(),
        status: TicketStatus::ToDo,
        priority: TicketPriority::Critical,
        assignee: Some("someone".to_string()),
        blockers: vec![blocker_id.clone()],
        tags: vec!["tag".to_string(), "extra".to_string()],
        spec_ref: Some(".lux/spec/b".to_string()),
        created_at: "2026-05-11T00:00:00Z".to_string(),
        updated_at: "2026-05-11T01:00:00Z".to_string(),
        ..Default::default()
    };
    let updated = store.update(&id, updated_ticket).unwrap();
    assert_eq!(updated.status, TicketStatus::ToDo);

    let blockers = store.check_blockers(&id).unwrap();
    assert_eq!(blockers.len(), 1);
    assert_eq!(blockers[0].id, blocker_id);

    store.delete(&id).unwrap();
    assert!(store.get(&id).unwrap().is_none());
}

#[test]
fn test_lux_ticket_id_uuid_format() {
    let ticket = ticket(
        &uuid::Uuid::new_v4().to_string(),
        TicketStatus::Backlog,
        vec![],
        vec![],
        None,
    );
    assert!(uuid::Uuid::parse_str(&ticket.id).is_ok());
}
