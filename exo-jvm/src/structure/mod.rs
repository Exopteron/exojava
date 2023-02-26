use std::{any::TypeId, collections::HashMap, fmt::Debug, mem::{size_of, align_of}};

use exo_class_file::item::ids::{
    field::{FieldDescriptor, FieldType},
    UnqualifiedName,
};
use nonmax::NonMaxU8;

use crate::{
    nugc::{
        collector::{
            AllocationError, GarbageCollector, GcObject, TheGc,
            Visitor, GcObjectVtable,
        },
        implementation::{GcPtr, NonNullGcPtr, VisitorTy},
    },
    value::{
        types::{ExactJavaType, FieldNameAndType, JavaTypes},
        Cast, JVMResult, JavaType,
    },
    vm::JVM,
};

/// A field definition.
pub struct FieldDef {
    pub ty: TypeId,
    pub nullable: bool,
    pub size: usize,
    pub align: usize,
    pub name: String,
    pub fns: GcObjectVtable
}

impl FieldDef {
    pub fn new<T: GcObject + Sized + 'static>(name: String) -> JVMResult<Self> {
        const {
            if T::DST {
                panic!("Must not be DST");
            }
        };
        Ok(Self {
            ty: TypeId::of::<T>(),
            nullable: T::NULLABLE,
            size: T::MIN_SIZE_ALIGN.0,
            align: T::MIN_SIZE_ALIGN.1,
            name,
            fns: T::vtable(),
        })
    }
    // pub fn new_java(gc: &JVM, descriptor: FieldNameAndType) -> JVMResult<Self> {

    //     fn wrap(gc: &JVM, f: &FieldType) -> JVMResult<ExactJavaType> {
    //         Ok(match f {
    //             exo_class_file::item::ids::field::FieldType::BaseType(v) => (*v).into(),
    //             exo_class_file::item::ids::field::FieldType::ObjectType(v) => ExactJavaType::ClassInstance(GcPtr::NULL),
    //             exo_class_file::item::ids::field::FieldType::ArrayType(ar) => {
    //                 let mut v = wrap(gc, &ar.0)?;
    //                 ExactJavaType::Array(v.cast(gc)?, ar.1)
    //             },
    //         })
    //     }

    //     Ok(Self {
    //         size: (descriptor.descriptor.as_ref()).size(),
    //         align: (descriptor.descriptor.as_ref()).align().get(),
    //         ty: FieldDefType::Java {
    //             name: descriptor.name,
    //             ty: wrap(gc, &descriptor.descriptor)?
    //         }
    //     })
    // }

    // pub fn new_native<T: 'static>(name: String) -> Self {
    //     Self {
    //         size: JavaTypes::Object.size(),
    //         align: JavaTypes::Object.align().get(),
    //         ty: FieldDefType::Native { name, ty: std::any::TypeId::of::<T>() , trace: |self_ptr, gc, tracer| {
    //             let v: &mut GcPtr<T> = unsafe { std::mem::transmute(self_ptr) };
    //             tracer.mark(gc, v);
    //         }}
    //     }
    // }
    // pub fn new_native_traced<T: 'static + Trace<TheGc>>(name: String) -> Self {
    //     Self {
    //         size: JavaTypes::Object.size(),
    //         align: JavaTypes::Object.align().get(),
    //         ty: FieldDefType::Native { name, ty: std::any::TypeId::of::<T>() , trace: |self_ptr, gc, tracer| {
    //             let v: &mut GcPtr<T> = unsafe { std::mem::transmute(self_ptr) };
    //             tracer.visit(gc, v);
    //             let mut this = v.get_mut(gc).unwrap();
    //             this.trace(gc, tracer);
    //         }}
    //     }
    // }
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

        let mut native_map = HashMap::new();

        // current offset in the structure
        let mut offset = 0;

        // largest alignment among all fields
        let mut largest_alignment = 0;

        let mut nullable_index = 0;

        for field in self.fields {
            let align = field.align;
            if align > largest_alignment {
                largest_alignment = align;
            }

            // the padding required for this field to be aligned at this offset
            let padding = (align - (offset % align)) % align;

            let idx = output_fields.len();
            native_map.insert(field.name, NativeFieldData {
                field_index: idx,
                nullable_index: Some(NonMaxU8::new(nullable_index).unwrap()),
                fns: field.fns
            });
            nullable_index += 1;
            if nullable_index > 64 {
                panic!("Too many non-nullable fields");
            }

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
            native_map,
            last_nullable_index: nullable_index
        }
    }
}

#[derive(Clone, Copy)]
pub struct NativeFieldData {
    pub nullable_index: Option<NonMaxU8>,
    pub field_index: usize,
    pub fns: GcObjectVtable
}

pub struct StructureDef {
    size: usize,
    align: usize,
    fields: Vec<OffsetSize>,
    last_nullable_index: u8,
    native_map: HashMap<String, NativeFieldData>,
}

unsafe impl GcObject for StructureDef {
    const MIN_SIZE_ALIGN: (usize, usize) = (size_of::<Self>(), align_of::<Self>());

    const NULLABLE: bool = false;

    const DST: bool = false;

    fn valid_dynamic_size(size: usize) -> bool {
        false
    }

    fn trace(
        &mut self,
        gc: &GarbageCollector,
        visitor: &mut VisitorTy,
    ) {
        
    }

    fn finalize(this: NonNullGcPtr<Self>, j: JVM) {
        
    }
}

// impl Finalize for StructureDef {
//     unsafe fn finalize(this: crate::nugc::implementation::NonNullGcPtr<Self>, j: crate::vm::JVM) {
//         std::ptr::drop_in_place(&mut *this.get_mut(&j.gc()));
//     }
// }

// impl Trace<TheGc> for StructureDef {
//     fn trace(
//         &mut self,
//         gc: &GarbageCollector<TheGc>,
//         visitor: &mut <TheGc as MemoryManager>::VisitorTy,
//     ) {
//         for (_, (_, v)) in self.java_map.iter_mut() {
//             visitor.visit_noref(gc, v);
//         }
//     }
// }

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
    // pub fn native_fields(&self) -> Vec<(String, (TypeId, OffsetSize, FieldTraceFn))> {
    //     let mut vec = vec![];
    //     for (k, v) in self.native_map.iter() {
    //         vec.push((k.clone(), (v.0, self.fields[v.1], v.2)));
    //     }
    //     vec
    // }


    pub fn native_field(&self, f: &str) -> Option<&NativeFieldData> {
        let idx = self.native_map.get(f)?;
        Some(idx)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct OffsetSize {
    pub offset: usize,
    pub size: usize,
}

// impl Debug for StructureDef {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         let mut s = f.debug_struct(&format!(
//             "BuiltStructure(size = {}, align = {})",
//             self.size, self.align
//         ));
//         if self.fields.is_empty() {
//             return s.finish();
//         } else if self.fields.len() == 1 {
//             s.field(self.ordered.get(&0).unwrap(), &self.fields[0]);
//             return s.finish();
//         }

//         let mut last_index = 0;
//         let last = self.fields[last_index];
//         let last_name = &self.ordered.get(&last_index).unwrap();
//         s.field(last_name, &last);
//         for i in 1..self.fields.len() {
//             let last = self.fields[last_index];
//             let current = self.fields[i];
//             let current_name = self.ordered.get(&i).map(|v| v.as_str()).unwrap_or("pad");

//             let pad_between = (last.offset + last.size) - current.offset;
//             if pad_between > 0 {
//                 s.field("pad", &pad_between);
//             }
//             s.field(current_name, &current);

//             last_index = i;
//         }
//         s.finish()
//     }
// }

#[cfg(test)]
mod tests {
    use super::{FieldDef, StructureBuilder};

    // #[test]
    // fn test_align() {
    //     let mut sbuilder = StructureBuilder::new();
    //     sbuilder.insert_field(FieldDef::new("byte".to_string(), 2, 1));
    //     sbuilder.insert_field(FieldDef::new("byte2".to_string(), 1, 1));
    //     sbuilder.insert_field(FieldDef::new("byte4".to_string(), 8, 1));

    //     let built = sbuilder.build();
    //     assert_eq!(built.size, 11);
    //     assert_eq!(built.align, 1);
    // }
}
