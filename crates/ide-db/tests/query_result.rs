use ide_db::{DbDriver, QueryResult};

#[test]
fn dirty_rows_and_pending_updates() {
    let mut q = QueryResult::empty();
    q.columns = vec!["id".into(), "name".into()];
    q.rows = vec![vec!["1".into(), "alice".into()]];
    q = q.with_meta(None, Some("users".into()), vec!["id".into()]);
    q.cursor_col = 1;
    q.set_cell("bob".into());
    assert_eq!(q.dirty_rows(), vec![0]);
    let stmts = q.pending_updates().expect("updates");
    assert_eq!(stmts.len(), 1);
    assert!(stmts[0].contains("UPDATE"));
    assert!(stmts[0].contains("name"));
}

#[test]
fn missing_composite_pk_column_bails() {
    let mut q = QueryResult::empty();
    q.columns = vec!["id".into(), "name".into()];
    q.rows = vec![vec!["1".into(), "alice".into()]];
    q = q.with_meta(
        Some("public".into()),
        Some("users".into()),
        vec!["tenant_id".into(), "id".into()],
    );
    q.cursor_col = 1;
    q.set_cell("bob".into());
    let err = q.pending_updates_for(DbDriver::Postgres).unwrap_err();
    assert!(
        err.to_string().contains("tenant_id") && err.to_string().contains("missing"),
        "{err:#}"
    );
}

#[test]
fn mysql_pending_update_uses_backticks() {
    let mut q = QueryResult::empty();
    q.columns = vec!["id".into(), "name".into()];
    q.rows = vec![vec!["1".into(), "alice".into()]];
    q = q.with_meta(Some("app".into()), Some("users".into()), vec!["id".into()]);
    q.cursor_col = 1;
    q.set_cell("bob".into());
    let stmts = q.pending_updates_for(DbDriver::Mysql).unwrap();
    assert!(stmts[0].contains("`app`.`users`"), "{}", stmts[0]);
    assert!(stmts[0].contains("`name` ="), "{}", stmts[0]);
    assert!(stmts[0].contains("`id` ="), "{}", stmts[0]);
}
