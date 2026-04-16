use std::io::Write;

use anyhow::Result;
use tabwriter::TabWriter;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListRow {
    pub saved: bool,
    pub status: &'static str,
    pub reason: String,
    pub repository: String,
    pub subject: String,
}

pub fn write_list(mut writer: impl Write, rows: &[ListRow]) -> Result<()> {
    if rows.is_empty() {
        writeln!(writer, "No notifications found.")?;
        return Ok(());
    }

    let mut table = TabWriter::new(Vec::new()).padding(2);
    writeln!(table, "SAVED\tSTATUS\tREASON\tREPOSITORY\tSUBJECT")?;

    for row in rows {
        let saved = if row.saved { "yes" } else { "no" };
        writeln!(
            table,
            "{saved}\t{}\t{}\t{}\t{}",
            row.status, row.reason, row.repository, row.subject
        )?;
    }

    table.flush()?;
    let rendered = String::from_utf8(table.into_inner()?)?;
    write!(writer, "{rendered}")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ListRow, write_list};

    #[test]
    fn renders_a_table() {
        let rows = vec![ListRow {
            saved: true,
            status: "unread",
            reason: "team_mention".to_owned(),
            repository: "cli/cli".to_owned(),
            subject: "PR #123  Example".to_owned(),
        }];
        let mut output = Vec::new();

        write_list(&mut output, &rows).expect("rendered table");

        let rendered = String::from_utf8(output).expect("utf8 output");
        assert!(rendered.contains("SAVED"));
        assert!(rendered.contains("team_mention"));
        assert!(rendered.contains("PR #123  Example"));
    }
}
