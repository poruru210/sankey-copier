use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct TicketMapper {
    // master_ticket -> slave_ticket (Active positions)
    active_map: HashMap<i64, i64>,
    // master_ticket -> pending_ticket (Pending orders)
    pending_map: HashMap<i64, i64>,
}

impl TicketMapper {
    pub fn new() -> Self {
        Self {
            active_map: HashMap::new(),
            pending_map: HashMap::new(),
        }
    }

    pub fn add_active(&mut self, master_ticket: i64, slave_ticket: i64) {
        self.active_map.insert(master_ticket, slave_ticket);
    }

    pub fn get_active(&self, master_ticket: i64) -> Option<i64> {
        self.active_map.get(&master_ticket).copied()
    }

    pub fn remove_active(&mut self, master_ticket: i64) -> Option<i64> {
        self.active_map.remove(&master_ticket)
    }

    pub fn add_pending(&mut self, master_ticket: i64, pending_ticket: i64) {
        self.pending_map.insert(master_ticket, pending_ticket);
    }

    pub fn get_pending(&self, master_ticket: i64) -> Option<i64> {
        self.pending_map.get(&master_ticket).copied()
    }

    pub fn remove_pending(&mut self, master_ticket: i64) -> Option<i64> {
        self.pending_map.remove(&master_ticket)
    }

    pub fn get_all_active_mappings(&self) -> Vec<(i64, i64)> {
        self.active_map.iter().map(|(&k, &v)| (k, v)).collect()
    }

    pub fn get_all_pending_mappings(&self) -> Vec<(i64, i64)> {
        self.pending_map.iter().map(|(&k, &v)| (k, v)).collect()
    }

    // Reverse lookup for pending orders (needed for OnTradeTransaction to find master ticket)
    pub fn get_master_from_pending(&self, pending_ticket: i64) -> Option<i64> {
        for (&m, &p) in &self.pending_map {
            if p == pending_ticket {
                return Some(m);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_active_mappings() {
        let mut mapper = TicketMapper::new();

        mapper.add_active(100, 200);
        assert_eq!(mapper.get_active(100), Some(200));
        assert_eq!(mapper.get_active(999), None);

        assert_eq!(mapper.remove_active(100), Some(200));
        assert_eq!(mapper.get_active(100), None);
    }

    #[test]
    fn test_pending_mappings() {
        let mut mapper = TicketMapper::new();

        mapper.add_pending(100, 300);
        assert_eq!(mapper.get_pending(100), Some(300));

        // Reverse lookup
        assert_eq!(mapper.get_master_from_pending(300), Some(100));
        assert_eq!(mapper.get_master_from_pending(999), None);

        assert_eq!(mapper.remove_pending(100), Some(300));
        assert_eq!(mapper.get_pending(100), None);
    }
}
