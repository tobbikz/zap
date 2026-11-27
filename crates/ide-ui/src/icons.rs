//! Glyphs for the Database studio.
//!
//! Three repertoires, picked from the host font:
//! - **Nerd** — Nerd Fonts v3 private-use icons (Linux default, or a Windows face whose name contains "Nerd").
//! - **Safe** — WGL4 / Consolas symbols. This is the Windows PowerShell default: those
//!   glyphs are one cell wide and actually exist in Consolas, so the layout does not
//!   shift into missing-glyph boxes.
//! - **Ascii** — `[theme] icons = false` / `"ascii"`.
//!
//! The app never draws emoji.

use crate::theme::{self, GlyphSet};
use ide_db::{DbDriver, DbNodeKind};

pub const CONNECTIONS: &str = "\u{f048d}";
pub const SERVER: &str = "\u{f233}";
pub const DATABASE: &str = "\u{f1c0}";
pub const SCHEMA: &str = "\u{f12e3}";
pub const TABLE: &str = "\u{f04eb}";
pub const VIEW: &str = "\u{f1094}";
pub const COLUMN: &str = "\u{f0835}";
pub const ADMIN: &str = "\u{f013}";
pub const QUERY: &str = "\u{f120}";
pub const ACTION: &str = "\u{f04b}";
pub const ROUTINE: &str = "\u{f121}";
pub const GROUP: &str = "\u{f07b}";
pub const SCRIPT: &str = "\u{f1c9}";
pub const PIN: &str = "\u{f08d}";
pub const KEY: &str = "\u{f084}";
pub const LINK: &str = "\u{f0c1}";
pub const PAGE_PREV: &str = "\u{f0d9}";
pub const PAGE_NEXT: &str = "\u{f0da}";
pub const RUN: &str = "\u{f04b}";
pub const EXPLAIN: &str = "\u{f0eb}";
pub const EXPORT: &str = "\u{f019}";
pub const APPLY: &str = "\u{f00c}";

fn pick(nerd: &'static str, safe: &'static str, ascii: &'static str) -> &'static str {
    match theme::glyph_set() {
        GlyphSet::Nerd => nerd,
        GlyphSet::Safe => safe,
        GlyphSet::Ascii => ascii,
    }
}

pub fn chevron_down() -> &'static str {
    // ▾ is not in Consolas; ▼ (U+25BC) is.
    pick("\u{25be}", "\u{25bc}", "v")
}

pub fn chevron_right() -> &'static str {
    // ▸ is not in Consolas; ► (U+25BA) is.
    pick("\u{25b8}", "\u{25ba}", ">")
}

pub fn run_glyph() -> &'static str {
    pick("\u{25b6}", "\u{25b6}", ">")
}

pub fn busy_glyph() -> &'static str {
    pick("\u{25cf}", "\u{25cf}", "*")
}

pub fn folder(expanded: bool) -> &'static str {
    match theme::glyph_set() {
        GlyphSet::Ascii => {
            if expanded {
                "[-]"
            } else {
                "[+]"
            }
        }
        _ => {
            if expanded {
                chevron_down()
            } else {
                chevron_right()
            }
        }
    }
}

/// Expand/collapse twist; leaves keep the same width.
pub fn twist(expandable: bool, expanded: bool) -> &'static str {
    if !expandable {
        " "
    } else if expanded {
        chevron_down()
    } else {
        chevron_right()
    }
}

/// The explorer glyph of a node, before its label.
pub fn node(kind: &DbNodeKind) -> &'static str {
    match kind {
        DbNodeKind::Group => pick(GROUP, "\u{25a0}", "[]"),
        DbNodeKind::Source => pick(SERVER, "\u{25a0}", "DB"),
        DbNodeKind::Database => pick(DATABASE, "\u{25cf}", "db"),
        DbNodeKind::Schema => pick(SCHEMA, "\u{25a1}", "sc"),
        DbNodeKind::Table => pick(TABLE, "\u{25aa}", "tb"),
        DbNodeKind::View => pick(VIEW, "\u{25c7}", "vw"),
        DbNodeKind::Column => pick(COLUMN, "\u{00b7}", "cl"),
        DbNodeKind::Admin => pick(ADMIN, "\u{2666}", "ad"),
        DbNodeKind::AdminQuery => pick(QUERY, ">", ">_"),
        DbNodeKind::AdminAction => pick(ACTION, "\u{25b6}", ">"),
        DbNodeKind::Routine => pick(ROUTINE, "f", "fn"),
    }
}

pub fn database() -> &'static str {
    pick(DATABASE, "\u{25cf}", "db")
}

/// The driver's glyph (fallback: `DB`).
pub fn driver(driver: DbDriver) -> &'static str {
    match theme::glyph_set() {
        GlyphSet::Ascii => "DB",
        GlyphSet::Safe => match driver {
            DbDriver::Postgres => "P",
            DbDriver::Mysql => "M",
            DbDriver::Sqlite => "S",
            DbDriver::Redis => "R",
            DbDriver::Mongo => "m",
            DbDriver::Elasticsearch => "E",
            DbDriver::Mssql => "Q",
            DbDriver::Clickhouse => "C",
            DbDriver::Duckdb => "D",
            DbDriver::Snowflake => "*",
            DbDriver::Bigquery => "B",
            DbDriver::Trino => "T",
        },
        GlyphSet::Nerd => match driver {
            DbDriver::Postgres => "\u{e76e}",
            DbDriver::Mysql => "\u{e704}",
            DbDriver::Sqlite => "\u{e7c4}",
            DbDriver::Redis => "\u{e76d}",
            DbDriver::Mongo => "\u{e7a4}",
            DbDriver::Elasticsearch => "\u{e7ca}",
            DbDriver::Mssql => "\u{e82e}",
            DbDriver::Clickhouse
            | DbDriver::Duckdb
            | DbDriver::Snowflake
            | DbDriver::Bigquery
            | DbDriver::Trino => DATABASE,
        },
    }
}

/// The glyph of a Structure section, from its heading (`COLUMNS (12)` → table_column).
/// Safe/Ascii: no cryptic `%` / `▪` prefixes — plain section words only (#24).
pub fn structure_section(text: &str) -> &'static str {
    if theme::glyph_set() != GlyphSet::Nerd {
        return "";
    }
    let head = text.trim_start().to_ascii_uppercase();
    if head.starts_with("COLUMNS") {
        COLUMN
    } else if head.starts_with("INDEXES") {
        "\u{f027b}"
    } else if head.starts_with("FOREIGN KEYS") {
        LINK
    } else if head.starts_with("TRIGGERS") {
        "\u{f140b}"
    } else if head.starts_with("ROUTINES") {
        ROUTINE
    } else if head.starts_with("GRANTS") {
        "\u{f132}"
    } else if head.starts_with("STATS") {
        "\u{f080}"
    } else if head.starts_with("ER") {
        "\u{f0e8}"
    } else if head.starts_with("DDL") || head.starts_with("SCHEMA DIFF") {
        "\u{f121}"
    } else if head.starts_with("ADMIN QUERY") {
        QUERY
    } else {
        ""
    }
}

pub fn connections() -> &'static str {
    pick(CONNECTIONS, "\u{25c6}", "DB")
}

pub fn page_arrows() -> (&'static str, &'static str) {
    match theme::glyph_set() {
        GlyphSet::Nerd => (PAGE_PREV, PAGE_NEXT),
        GlyphSet::Safe => ("\u{2039}", "\u{203a}"),
        GlyphSet::Ascii => ("<", ">"),
    }
}

pub fn pin() -> &'static str {
    pick(PIN, "*", "*")
}

pub fn key() -> &'static str {
    pick(KEY, "#", "PK")
}

pub fn link() -> &'static str {
    pick(LINK, "~", "FK")
}

pub fn script() -> &'static str {
    pick(SCRIPT, "\u{00b6}", "sql")
}

/// File-tree glyph. One cell in Safe/Ascii so a missing Nerd Font cannot shift the row.
pub fn file(name: &str, is_dir: bool, expanded: bool) -> &'static str {
    if is_dir {
        return match theme::glyph_set() {
            GlyphSet::Nerd => {
                if expanded {
                    "\u{e5fe}"
                } else {
                    "\u{e5ff}"
                }
            }
            GlyphSet::Safe => {
                if expanded {
                    "\u{25a0}"
                } else {
                    "\u{25a1}"
                }
            }
            GlyphSet::Ascii => {
                if expanded {
                    "-"
                } else {
                    "+"
                }
            }
        };
    }
    match theme::glyph_set() {
        GlyphSet::Ascii => ".",
        GlyphSet::Safe => safe_file(name),
        GlyphSet::Nerd => nerd_file(name),
    }
}

fn safe_file(name: &str) -> &'static str {
    let lower = name.to_ascii_lowercase();
    match lower.as_str() {
        "cargo.toml" | "cargo.lock" => "C",
        "readme.md" | "license" | "license.md" => "\u{00b6}",
        ".gitignore" | ".gitattributes" => "\u{00b7}",
        _ => {
            let ext = lower.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
            match ext {
                "rs" => "R",
                "toml" => "#",
                "md" | "markdown" => "\u{00b6}",
                "json" | "jsonc" => "{",
                "yml" | "yaml" => "Y",
                "ts" | "tsx" | "js" | "jsx" | "mjs" => "J",
                "py" => "P",
                "go" => "G",
                "lua" => "L",
                "sh" | "bash" | "zsh" | "ps1" => "$",
                "sql" => "S",
                "html" | "htm" | "css" | "scss" => "H",
                "svg" | "png" | "jpg" | "jpeg" | "webp" | "gif" => "I",
                "lock" => "#",
                _ => "\u{00b7}",
            }
        }
    }
}

fn nerd_file(name: &str) -> &'static str {
    let lower = name.to_ascii_lowercase();
    match lower.as_str() {
        "cargo.toml" | "cargo.lock" => "\u{e7a8}",
        "readme.md" | "license" | "license.md" => "\u{f48a}",
        ".gitignore" | ".gitattributes" => "\u{f1d3}",
        _ => {
            let ext = lower.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
            match ext {
                "rs" => "\u{e7a8}",
                "toml" => "\u{e6b2}",
                "md" | "markdown" => "\u{f48a}",
                "json" | "jsonc" => "\u{e60b}",
                "yml" | "yaml" => "\u{f481}",
                "ts" | "tsx" => "\u{e628}",
                "js" | "jsx" | "mjs" => "\u{e74e}",
                "py" => "\u{e73c}",
                "go" => "\u{e724}",
                "lua" => "\u{e620}",
                "sh" | "bash" | "zsh" => "\u{f489}",
                "sql" => "\u{e706}",
                "html" | "htm" => "\u{e736}",
                "css" | "scss" => "\u{e749}",
                "svg" | "png" | "jpg" | "jpeg" | "webp" | "gif" => "\u{f1c5}",
                "lock" => "\u{f023}",
                _ => "\u{f016}",
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ide_db::DbNodeKind;
    use unicode_width::UnicodeWidthStr;

    #[test]
    fn safe_repertoire_is_single_cell() {
        theme::set_glyph_set(GlyphSet::Safe);
        theme::set_block_glyphs(false);
        let samples = [
            connections(),
            database(),
            node(&DbNodeKind::Table),
            node(&DbNodeKind::View),
            node(&DbNodeKind::Column),
            node(&DbNodeKind::Admin),
            node(&DbNodeKind::Routine),
            chevron_down(),
            chevron_right(),
            run_glyph(),
            busy_glyph(),
            pin(),
            key(),
            link(),
            script(),
            file("src", true, false),
            file("query.sql", false, false),
            file("readme.md", false, false),
            theme::caret(),
            theme::select_mark(),
        ];
        for s in samples {
            assert_eq!(UnicodeWidthStr::width(s), 1, "{s:?}");
        }
        let (prev, next) = page_arrows();
        assert_eq!(UnicodeWidthStr::width(prev), 1, "{prev:?}");
        assert_eq!(UnicodeWidthStr::width(next), 1, "{next:?}");
        theme::set_glyph_set(GlyphSet::Nerd);
        theme::set_block_glyphs(true);
    }
}
