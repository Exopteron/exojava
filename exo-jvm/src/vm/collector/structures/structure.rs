use std::{any::TypeId, collections::HashMap, fmt::Debug, mem::{size_of, align_of}, ptr::NonNull, alloc::Layout, sync::atomic::Ordering};

use exo_class_file::item::ids::{
    field::{FieldDescriptor, FieldType},
    UnqualifiedName,
};
use nonmax::NonMaxU8;

use crate::vm::collector::{GcRootVTable, object::{GcObject, Trace}, GcRootMeta};

use super::{GcRef, reference::{store_raw, load_raw}};

/// A field definition.
pub struct FieldDef {
    pub ty: TypeId,
    pub size: usize,
    pub align: usize,
    pub name: String,
    pub fns: GcRootVTable
}



impl FieldDef {
    pub fn new<T: GcObject + Copy + Sized + 'static>(name: String) -> Self {
        Self {
            ty: TypeId::of::<T>(),
            size: size_of::<T>(),
            align: align_of::<T>(),
            name,
            fns: GcRootVTable::new::<T>(),
        }
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

    pub fn add_field(mut self, f: FieldDef) -> Self {
        self.fields.push(f);
        self
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
                fns: field.fns
            });

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
        }
    }
}

#[derive(Clone, Copy)]
pub struct NativeFieldData {
    pub field_index: usize,
    pub fns: GcRootVTable
}

pub struct StructureDef {
    size: usize,
    align: usize,
    fields: Vec<OffsetSize>,
    native_map: HashMap<String, NativeFieldData>,
}

impl StructureDef {
    pub fn layout(&self) -> Layout {
        Layout::from_size_align(self.size, self.align).unwrap()
    }
}

super::super::object::bare_impl!(StructureDef);
#[repr(C)]
pub struct StructureMetadata {
    pub def: GcRef<StructureDef>,
    pub offset: usize
}

pub fn structure_vtable() -> GcRootVTable {
    GcRootVTable { needs_traced: true, tracer: |visitor, gc, obj| {
        unsafe {
            let data: *mut StructureMetadata = obj as *mut StructureMetadata;
            visitor.visit_noref(gc, &mut (*data).def);
            let real_offset = (*data).offset;
            (*data).def.update(|v| {
                let mut fields = vec![];
                
                
                let base: *mut u8 = (data as *mut u8).add(real_offset);
                for (_, f) in &v.native_map {


                    let f_data = v.fields[f.field_index];
                    (f.fns.tracer)(visitor, gc, base.add(f_data.offset) as *mut ());

                    fields.push((f.fns, v.fields[f.field_index]));
                }    
            });
            // TODO: trace fields
        }
    }, finalizer: |_, _, _, _, _| {}, dropper: |v| {

    } }
}

pub struct Structure {
    meta: NonNull<GcRootMeta>,
}
unsafe impl Trace for Structure {
    const NEEDS_TRACED: bool = true;

    fn trace(
        &mut self,
        gc: &mut crate::vm::collector::gc::VMGcState,
        visitor: &mut crate::vm::collector::object::VisitorImpl,
    ) {
        unsafe {
            visitor.visit(gc, self.meta.as_mut());
        }
    }
}
unsafe impl GcObject for Structure {
    
}

impl Structure {
    pub fn new(meta: NonNull<GcRootMeta>) -> Self {
        Self {
            meta
        }
    }
    pub fn field_offset(&mut self, name: &str) -> usize {
        unsafe {
            let meta = self.meta.as_mut().data_ptr_mut::<StructureMetadata>();
            let field_loc = (*meta).def.update(|def| {
                let i = def.native_map.get(name).unwrap().field_index;
                def.fields[i].offset
            });
            (*meta).offset + field_loc
        }
    }
    pub unsafe fn store<T: GcObject + Copy + 'static>(&mut self, off: usize, volatile: bool, v: T) {
        let ordering = match volatile {
            true => Ordering::SeqCst,
            false => Ordering::Relaxed,
        };
        unsafe {
            store_raw((self.meta.as_ptr() as *mut u8).add(off) as *mut  T, ordering, v);
        }
    }

    pub unsafe fn load<T: GcObject + Copy + 'static>(&mut self, off: usize, volatile: bool) -> T {
        let ordering = match volatile {
            true => Ordering::SeqCst,
            false => Ordering::Relaxed,
        };
        unsafe {
            load_raw((self.meta.as_ptr() as *mut u8).add(off) as *mut  T, ordering)
        }
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
