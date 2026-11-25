# Changelog

## Unreleased

### Added

- SQL completions cache columns for non-active schemas; `schema.table.` completes columns without switching the chip (#33).
- Results grid: `S` toggles client-side sort (ASC → DESC → off) on the focused column; sort indicator in header (#31).
- Results grid: `>` / `<` widen / narrow the focused column by 4 chars (#31).
- Studio mouse: click to focus panes, click explorer rows to select, wheel scrolls Explorer / Structure / Console / Results (#27).
- Console horizontal scroll for long SQL lines; cursor past visible columns scrolls with `▶` overflow marker (#26).
- SQL tab management keys (`t`/`+` new outside Results, `[`/`]` cycle, `x`/`Ctrl-W` close) shown in Discover and footer hints (#25).
- Run statement at cursor: `Alt-Enter` / `Shift-F5` runs only the `;`-delimited statement under the caret, or the selection (#20).

- Charts: `c` chart · `C` cycle kind (bar/line/pie) · `V` cycle value column · `L` cycle category column in Results (#110).
- Transaction chords in Results: `t` begin · `x` rollback · `X` commit; `+` still opens a SQL tab (#28).

### Fixed

- SQL completions: stop rewriting the footer status on every keystroke; popup is sufficient (#32).
- Schema diff: document scope as table + column name/type/null only; note indexes/FKs/constraints not compared (#124).
- MSSQL dump: prefer `mssql-scripter` over broken `sqlcmd SELECT *` grid path that produced non-restorable output (#109).
- MSSQL fallback dump: include full type specs (varchar length, decimal precision/scale), N-prefix string literals, GO batch separators (#113).
- dbt/seed/golden: `seed_fixtures` concatenates all `testdata/*.sql` in sorted order instead of silently keeping only the last file; `golden_diff` picks the newest CSV in `testdata/golden/` instead of hardcoding `latest.csv`; persona help notes dbt commands are recipes, schema diff is table+col only (#144).
- Data compare: deterministic ORDER BY (PK or col 1); don't collapse duplicate rows without PK; fix ClickHouse db.table resolution (#137).
- `generate_schema_diff_ddl`: strip `?` nullable markers from DDL types; patch left (target) side for `+` objects; dialect-valid DROP (no CASCADE on SQLite/MSSQL); nullability drift emits SET/DROP NOT NULL (#134).
- Structure pane: separate `structure_selected` from `structure_scroll`; ↑↓ moves selection with highlight; Enter acts on selected row (#29).
- FK jump from Structure auto-expands target schema if not loaded (#30).
- Postgres EXPLAIN uses plan-only `EXPLAIN (FORMAT TEXT)` instead of `EXPLAIN ANALYZE` to avoid executing DML/DDL (#38).
- Auto-paging and table-open SQL use `SELECT TOP` / `OFFSET FETCH` on MSSQL instead of illegal `LIMIT` (#37).
- Copy connection URI to clipboard from Explorer palette command (#35).
- Collapse-all (`c`) and collapse-others (`C`) actions in the database explorer tree (#34).
- Query history and bookmarks filter by active `source_id`; cross-engine entries still visible when no source is active (#45).
- ER sketch and FK introspection implemented for MSSQL (`sys.foreign_keys`) and DuckDB (`duckdb_constraints()`) (#117).
- Panic hook restores terminal raw mode and alternate screen on crash; user shell stays usable (#75).
- Windows truecolor: detect WT_SESSION / COLORTERM and enable full 24-bit palettes instead of forcing 16-color (#74).
- Agents/Sessions Windows defaults: use `powershell`/`python`/`where.exe`/`mstsc` on Windows; `which()` checks PATHEXT (#65).
- Default workspace avoids Downloads/Desktop/tmp; strip `\\?\` extended-length prefix from canonicalized paths (#87).
- Clipboard copy/paste retries on transient `OpenClipboard` failures; OSC 52 fallback for SSH/WSL (#161).
- Agents/Sessions keep current product mode (Database) instead of forcing Code; `db_agent_seed` always provides DB context when a source is connected (#90).
- BigQuery: poll `jobComplete` until done (~60 s timeout); fix `test_data_source` pool key mismatch (#157).
- Snowflake: `snowflake_configured` accepts env-only host/user; `output_format=csv`; CSV fallback; first-seen column order (#158).
- Redis/Mongo/ES: `guard_writable` allows read commands; Redis keys carry type-aware `query_sql`; `open_selected_table_tab` uses it; driver-specific studio starter (#91).
- PG ER: join FK columns via `referential_constraints` + `position_in_unique_constraint` for correct composite-FK mapping; ER source_table set to None (#98).
- `split_sql_script`: skip `--`/`/* */` comments outside strings; backslash-quote in non-MSSQL; track string/comment state for MSSQL `GO` (#120).
- Schema diff: reject SQLite cross-schema diff; ClickHouse single-part spec treated as database (#135).
- Airflow/BI/model deep-links use `open_browser` (Windows/macOS/Linux) (#61 #49).
- Results `P` cycles pinned columns (#44).
- FEATURES: honest completions/search scope; migration preview (not dry-run) (#47 #142).
- Data compare status notes `≤500 rows/side` (#123).
- Migration rollback skips journal rollbacks / `.down.sql` (no `.down.down`) (#92).
- `install.ps1` fails clearly when `PSScriptRoot` empty under `irm|iex` (#69).
- Terminal forwards Ctrl-* to the focused PTY; Studio terminal preferred (#145).
- Results meta shows query timeout; palette cycles 30s…off (#50).
- Kill session confirms foreign pids; help distinguishes Esc cancel vs K (#54).
- Home Recents lists recent tables (fallback: queries) (#55).
- Studio `?` help matches live keymap (no Code-only Super chords) (#39).
- Empty SELECT keeps column headers via PG prepare; no meta on 0-col (#43).
- MSSQL EXPLAIN streams SHOWPLAN rows and always clears SHOWPLAN_ALL (#57 #52).
- New connections default `ssl_trust_cert` off; form labels trust as insecure (#48).
- `split_statements` respects dollar-quotes and SQL comments (#51).
- `parse_select_target` handles quoted ids / subqueries; JOIN still refuses Apply (#53).
- Fullscreen follows focus or clears on Home / product / non-pane focus (#67).
- Home/Workspaces open 10th+ connections by unbounded index (#79).
- Fan-out assert queries pinned sources; tab binding restored (#88).
- Studio Terminal forwards keys before pane chords (#86).
- DB Home: Tab cycles Connections/Recents/Favorites + health badges (#78).
- Connect form scrolls to focused SSH fields (#96).
- MSSQL cells render uuid/decimal/date/bytes (not silent NULL) (#56).
- Results: `m` opens actions; `[`/`]` page; monitor is Shift-M (#80).
- Host chords skip forms; Ctrl-Shift-A/O wire Search/Outline (#95).
- Inspect Chart: ↑↓ move rows; header stays pinned (#118).
- ClickHouse strips trailing `;` and respects explicit `FORMAT` (#66).
- Query params: bare numbers, no Redis false positives, safer replace (#68).
- MSSQL grid edit gets primary keys via `sys.indexes` (#60).
- `split_sql_script` preserves SQL escaped quotes (`O''Brien`) (#97).
- ER navigate works for canvas; jump scoped to active source Table|View; Structure `… +N more` (#116).
- Structure Enter jumps FK targets (ToggleOrOpen → structure_confirm) (#93).
- Charts: anchor bars at 0, respect cursor col, Safe pie glyphs, downsample sparklines (#119).
- Chart `#row` labels keep grid indices when NULLs are skipped (#104).
- Results reserves height for inline chart; `m` → Hide chart (#105).
- Postgres ER/FK: join on `constraint_schema`; cross-schema edges keep `schema.table` (#114).
- SQLite ER: escape PRAGMA idents; resolve NULL `to` to parent PK (#107).
- ER canvas draws all schema tables, not only FK participants (#106).
- Pie chart: abs-normalize + clamp wedges (no OOM on mixed signs) (#103).
- ClickHouse dump: views are DDL-only; SQLInsert names real `` `db`.`table` `` (#101 #112).
- Create Table: AddBtn prev-field, Tab wraps, Delete guarded while typing, scroll (#149).
- Trino: nextUri Basic Auth, DELETE on early stop, `OFFSET…LIMIT` paging (#155).
- Elasticsearch: Explorer → `_search`; METHOD+body; tabular JSON arrays; unwrap `_id` (#156).
- DuckDB dates/decimals/timestamps render as ISO/decimal (not Debug) (#159).
- CSV/JSON import: driver-aware quoting + batched multi-row INSERT (#130).
- ER canvas: Unicode width + aligned box borders (#115).
- MSSQL ALTER ADD omits illegal `COLUMN`; `sp_rename` includes table (#127).
- Trino sends Console schema as `X-Trino-Schema` (#129).
- Admin Blocking resolves columns by name; scroll in card line units (#132).
- MySQL/MariaDB Health uses `information_schema` (no `performance_schema` required) (#133).
- CreateObject: mask Role passwords; ClickHouse Role fields; MSSQL Function uses Body (#150).
- WHERE/ORDER wells: Esc restores snapshot; caret stays visible; empty Tab switches wells (#152).
- CLI `--version` / `--help` exit cleanly; FEATURES docs pin **v0.6.3** (#141).
- Privilege / RLS inspect quotes schema/table as SQL literals (#122).
- Save as view qualifies with Console schema and uses `CREATE OR REPLACE` (#125).
- Elasticsearch/Trino HTTP honors SSH `local_port` and resolves vault passwords (#128).
- Settings editor: cursor navigation, Enter = newline, Ctrl-S saves (#148).
- Bookmarks: manage/delete/rename from palette; warn when FIFO-dropping past 40 (#163).

## [0.6.2] - 2026-11-19

### Added

- TUI design language port from Hop: tuile `Role` tokens, footer key chips, jackin L0 fill, opaque modal scrim, restrained busy spinner (`ZAP_ANIMATIONS=0` to disable). Multi-theme DataRig palettes stay unlocked. See [docs/TUI-DESIGN.md](docs/TUI-DESIGN.md).
- Inspect Chart keyboard controls: `c` cycles bar/line/pie, `[` `]` adjusts visible rows.

### Changed

- Default `dark` theme retuned to DataRig’s teal accent / quiet borders / selection tints; Studio panes, Console Run chip, active tabs, and Results cursor follow that chrome.
- Console F5 chip: filled accent when Console is focused, accent-ink when idle.
- Results: no row-index gutter; caret is a solid cell block; `where` / `order` use accent labels with brackets; cell peek shows value only.
- Inspect modal: quieter chrome, rail + value-only selection (no full-width wash).
- Charts: cleaner spark/bars; kind and depth are controllable from Inspect → Chart.

### Fixed

- AMOLED `black`/`white` restored to a pure canvas (no grey pane/toolbar wash); invert selection stays readable. DataRig surface hierarchy applies to chromatic themes only.
- Results/F5 accent fills only when the accent is chromatic — mono no longer washes to white-on-white.

## [0.6.1] - 2026-11-18

### Added

- GitHub Release binaries: `zap-linux-x86_64` and `zap-windows-x86_64.exe`. Install scripts download them and only compile from source if the asset is missing.

### Fixed

- Windows PowerShell: the console is switched to UTF-8, raster fonts are upgraded to Consolas for the session, and the UI uses Consolas glyphs (square borders, `|` caret, no Nerd Font private-use icons) so the layout stays aligned. Windows Terminal with Cascadia keeps rounded borders and block carets. A Nerd Font face, `[theme] icons = "nerd"`, or `ZAP_ICONS=nerd` restores the full icon set.

## [0.6.0] - 2026-11-15

### Changed

- **Rebrand to Zap** — display name **Zap** (capital Z), binary `zap`, config `~/.config/zap` (falls back to existing `~/.config/tui-ide`), GitHub `tobbikz/zap`.
