use super::Hwp3Error;

const DEFAULT_MAX_DEPTH: usize = 32;
const DEFAULT_MAX_NODES: usize = 262_144;
const DEFAULT_MAX_TABLE_GRID_SLOTS: usize = 262_144;

#[derive(Debug, Clone, Copy)]
pub struct Hwp3Limits {
    pub max_decoded_bytes: usize,
    pub max_depth: usize,
    pub max_nodes: usize,
    pub max_table_grid_slots: usize,
}

impl Default for Hwp3Limits {
    fn default() -> Self {
        Self {
            max_decoded_bytes: super::HWP3_MAX_RECORD_SIZE,
            max_depth: DEFAULT_MAX_DEPTH,
            max_nodes: DEFAULT_MAX_NODES,
            max_table_grid_slots: DEFAULT_MAX_TABLE_GRID_SLOTS,
        }
    }
}

pub(crate) struct ParseContext {
    limits: Hwp3Limits,
    depth: usize,
    nodes: usize,
    table_grid_slots: usize,
}

impl ParseContext {
    pub(crate) fn new(limits: Hwp3Limits) -> Self {
        Self {
            limits,
            depth: 0,
            nodes: 0,
            table_grid_slots: 0,
        }
    }

    pub(crate) fn nested<T>(
        &mut self,
        parse: impl FnOnce(&mut Self) -> Result<T, Hwp3Error>,
    ) -> Result<T, Hwp3Error> {
        if self.depth >= self.limits.max_depth {
            return Err(Hwp3Error::LimitExceeded {
                message: "HWP3 nesting exceeds the parse budget".to_owned(),
            });
        }
        self.depth += 1;
        let result = parse(self);
        self.depth -= 1;
        result
    }

    pub(crate) fn node(&mut self) -> Result<(), Hwp3Error> {
        self.nodes = self
            .nodes
            .checked_add(1)
            .filter(|count| *count <= self.limits.max_nodes)
            .ok_or_else(|| Hwp3Error::LimitExceeded {
                message: "HWP3 node count exceeds the parse budget".to_owned(),
            })?;
        Ok(())
    }

    pub(crate) fn table(&mut self, rows: u16, columns: u16) -> Result<(), Hwp3Error> {
        let slots = (usize::from(rows.max(1)) + 1)
            .checked_mul(usize::from(columns.max(1)) + 1)
            .ok_or_else(|| Hwp3Error::LimitExceeded {
                message: "HWP3 table dimensions overflow".to_owned(),
            })?;
        self.table_grid_slots = self
            .table_grid_slots
            .checked_add(slots)
            .filter(|slots| *slots <= self.limits.max_table_grid_slots)
            .ok_or_else(|| Hwp3Error::LimitExceeded {
                message: "HWP3 aggregate table grid exceeds the parse budget".to_owned(),
            })?;
        Ok(())
    }
}
