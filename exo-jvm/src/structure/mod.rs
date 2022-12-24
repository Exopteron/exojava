use std::{collections::HashMap, fmt::Debug};


/// A field definition.
#[derive(Debug)]
pub struct FieldDef {
    pub name: String,
    pub size: usize,
    pub align: usize,
}

impl FieldDef {
    pub fn new(name: String, size: usize, align: usize) -> Self {
        Self { name, size, align }
    }
}

/// Helper for constructing structures.
#[derive(Default)]
pub struct StructureBuilder {
    pub fields: Vec<FieldDef>,
}

impl StructureBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_field(&mut self, f: FieldDef) {
        self.fields.push(f);
    }

    pub fn build(mut self) -> StructureDef {
        // sorted by size
        self.fields.sort_by(|a, b| b.size.cmp(&a.size));

        let mut output_fields = vec![];

        let mut map = HashMap::new();
        let mut ordered = HashMap::new();

        // current offset in the structure
        let mut offset = 0;

        // largest alignment among all fields
        let mut largest_alignment = 0;


        for field in self.fields {
            
            
            let align = field.align;
            if align > largest_alignment {
                largest_alignment = align;
            }

            // the padding required for this field to be aligned at this offset
            let padding = (align - (offset % align)) % align;

            let idx = output_fields.len();
            ordered.insert(idx, field.name.clone());
            map.insert(field.name, idx);

            output_fields.push(OffsetSize {
                offset,
                size: field.size,
            });

            if padding > 0 {
                // add the padding as a field
                output_fields.push(OffsetSize {
                    offset: offset + field.size,
                    size: padding,
                });
            }

            offset += field.size + padding;
        }

        let structure_align = largest_alignment;
        let v = offset % structure_align;
        if v != 0 {
            let end_padding = structure_align - v;

            output_fields.push(OffsetSize {
                offset,
                size: end_padding,
            });
            offset += end_padding;
        }

        StructureDef {
            size: offset,
            align: structure_align,
            fields: output_fields,
            map,
            ordered,
        }
    }
}

pub struct StructureDef {
    size: usize,
    align: usize,
    fields: Vec<OffsetSize>,
    map: HashMap<String, usize>,
    ordered: HashMap<usize, String>,
}

impl StructureDef {
    pub fn size(&self) -> usize {
        self.size
    }

    pub fn align(&self) -> usize {
        self.align
    }

    pub fn fields(&self) -> &[OffsetSize] {
        &self.fields
    }

    pub fn field_offset(&self, f: &str) -> Option<OffsetSize> {
        let idx = self.map.get(f)?;
        self.fields.get(*idx).copied()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct OffsetSize {
    pub offset: usize,
    pub size: usize,
}

impl Debug for StructureDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct(&format!(
            "BuiltStructure(size = {}, align = {})",
            self.size, self.align
        ));
        if self.fields.is_empty() {
            return s.finish();
        } else if self.fields.len() == 1 {
            s.field(self.ordered.get(&0).unwrap(), &self.fields[0]);
            return s.finish();
        }

        let mut last_index = 0;
        let last = self.fields[last_index];
        let last_name = &self.ordered.get(&last_index).unwrap();
        s.field(last_name, &last);
        for i in 1..self.fields.len() {
            let last = self.fields[last_index];
            let current = self.fields[i];
            let current_name = self.ordered.get(&i).map(|v| v.as_str()).unwrap_or("pad");

            let pad_between = (last.offset + last.size) - current.offset;
            if pad_between > 0 {
                s.field("pad", &pad_between);
            }
            s.field(current_name, &current);

            last_index = i;
        }
        s.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::{FieldDef, StructureBuilder};

    #[test]
    fn test_align() {
        let mut sbuilder = StructureBuilder::new();
        sbuilder.insert_field(FieldDef::new("byte".to_string(), 2, 1));
        sbuilder.insert_field(FieldDef::new("byte2".to_string(), 1, 1));
        sbuilder.insert_field(FieldDef::new("byte4".to_string(), 8, 1));

        let built = sbuilder.build();
        assert_eq!(built.size, 11);
        assert_eq!(built.align, 1);
    }
}
