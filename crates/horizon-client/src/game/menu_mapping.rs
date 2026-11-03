//! Mapping between the verified menu layout and game slot ids.
#[derive(Clone, Copy)]
pub(super) struct Map {
    pub(super) container: usize,
    pub(super) menu_container: usize,
    pub(super) table: bool,
    pub(super) cells: usize,
    pub(super) plain: bool,
}
impl Map {
    fn main(&self) -> usize {
        match (self.plain, self.table) {
            (false, _) => self.menu_container,
            (true, true) => 10,
            (true, false) => 9,
        }
    }
    pub(super) fn to_menu(self, g: usize) -> Option<usize> {
        let main = self.main();
        if self.plain {
            match g {
                200 => Some(0),
                100..109 if g - 100 < self.cells => Some(1 + g - 100),
                500..504 if !self.table => Some(5 + g - 500),
                504 if !self.table => Some(45),
                9..36 => Some(main + g - 9),
                0..9 => Some(main + 27 + g),
                _ => None,
            }
        } else {
            match g {
                300.. if g < 300 + self.container => Some(g - 300),
                9..36 => Some(main + g - 9),
                0..9 => Some(main + 27 + g),
                _ => None,
            }
        }
    }
    pub(super) fn to_game(self, i: usize) -> Option<usize> {
        let main = self.main();
        if self.plain {
            match i {
                0 => None,
                1..=9 if i - 1 < self.cells => Some(100 + i - 1),
                5..9 if !self.table => Some(500 + i - 5),
                45 if !self.table => Some(504),
                _ if i >= main && i < main + 27 => Some(9 + i - main),
                _ if i >= main + 27 && i < main + 36 => Some(i - main - 27),
                _ => None,
            }
        } else if i < self.container {
            Some(300 + i)
        } else if i >= main && i < main + 27 {
            Some(9 + i - main)
        } else if i >= main + 27 && i < main + 36 {
            Some(i - main - 27)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Map;

    #[test]
    fn partial_rows_never_alias_player_inventory_or_padding() {
        for count in [1usize, 2, 9, 15, 27, 54] {
            let padded = count.div_ceil(9) * 9;
            let map = Map {
                container: count,
                menu_container: padded,
                table: false,
                cells: 0,
                plain: false,
            };
            for slot in 0..count {
                assert_eq!(map.to_game(slot), Some(300 + slot));
            }
            for slot in count..padded {
                assert_eq!(map.to_game(slot), None);
            }
            for game in (0..36).chain(300..300 + count) {
                let menu = map.to_menu(game).unwrap();
                assert_eq!(
                    map.to_game(menu),
                    Some(game),
                    "{count} physical slots, game slot {game}"
                );
                assert_eq!(
                    (0..padded + 36)
                        .filter(|&slot| map.to_game(slot) == Some(game))
                        .count(),
                    1
                );
            }
            assert_eq!(map.to_game(padded + 36), None);
        }
    }
}
