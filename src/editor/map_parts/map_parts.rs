use super::MapPart;
use crate::editor::{Error, Result, Transform};

#[derive(Debug, Clone)]
pub struct MapParts(Vec<MapPart>);

impl MapParts {
    /// Merge all map parts into a single part
    /// If new_name is some, then the new name is applied, else the name of the first map part is kept
    pub fn merge_all_parts(&mut self, new_name: Option<String>) {
        if self.0.is_empty() {
            return;
        }

        while self.0.len() > 1 {
            let part = self.0.pop().unwrap();

            self.0[0].merge(part);
        }

        if let Some(name) = new_name {
            self.0[0].name = name;
        }
    }

    /// Merge two of the map parts.
    /// The second part is merged into the first part. The name of the first part is kept
    /// The order of parts is also kept
    pub fn merge_two_parts(&mut self, part_1_index: usize, part_2_index: usize) -> Result<()> {
        if part_1_index >= self.num_map_parts()
            || part_2_index >= self.num_map_parts()
            || part_1_index == part_2_index
        {
            Err(Error::MapPartMergeError)
        } else {
            let part2 = self.0.remove(part_2_index);

            self.0[part_1_index].merge(part2);
            Ok(())
        }
    }

    pub fn remove_map_part_by_index(&mut self, index: usize) -> Option<MapPart> {
        if index < self.num_map_parts() {
            Some(self.0.remove(index))
        } else {
            None
        }
    }

    /// Case sensitive
    pub fn get_map_part_by_name(&self, name: &str) -> Option<&MapPart> {
        self.0.iter().find(|p| p.name.as_str() == name)
    }

    /// Case sensitive
    pub fn get_map_part_by_name_mut(&mut self, name: &str) -> Option<&mut MapPart> {
        self.0.iter_mut().find(|p| p.name.as_str() == name)
    }

    pub fn get_map_part_by_index(&self, index: usize) -> Option<&MapPart> {
        if index >= self.0.len() {
            None
        } else {
            Some(&self.0[index])
        }
    }

    pub fn get_map_part_by_index_mut(&mut self, index: usize) -> Option<&mut MapPart> {
        if index >= self.0.len() {
            None
        } else {
            Some(&mut self.0[index])
        }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, MapPart> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, MapPart> {
        self.0.iter_mut()
    }

    pub fn num_map_parts(&self) -> usize {
        self.0.len()
    }
}

impl MapParts {
    pub(crate) fn write<W: std::io::Write>(
        self,
        write: &mut W,
        transform: &Transform,
    ) -> std::result::Result<(), std::io::Error> {
        write.write_all(format!("<parts count=\"{}\" current\"0\">\n", self.0.len()).as_bytes())?;

        for part in self.0 {
            part.write(write, transform)?;
        }

        write.write_all("</parts>\n".as_bytes())
    }
}
