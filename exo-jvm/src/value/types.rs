use exo_class_file::item::ids::{UnqualifiedName, field::{FieldDescriptor, FieldType, BaseType}, method::{MethodDescriptor, MethodName}};

use crate::nugc::{implementation::{GcPtr, OwnedGcPtr}, collector::{Structure, Trace, TheGc, Visitor}};

use super::JavaType;

pub type JByte = i8;
pub type JShort = i16;
pub type JInt = i32;
pub type JLong = i64;

pub type JChar = u16;
pub type JFloat = f32;
pub type JDouble = f64;
pub type JBoolean = bool;

macro_rules! rust_type_impl {
    ($v:ty) => {
        impl JavaType for $v {
            fn size(&self) -> usize {
                std::mem::size_of::<$v>()
            }

            fn align(&self) -> std::num::NonZeroUsize {
                std::num::NonZeroUsize::new(std::mem::align_of::<$v>()).unwrap()
            }
        }
    };
}

rust_type_impl!(JByte);
rust_type_impl!(JShort);
rust_type_impl!(JInt);
rust_type_impl!(JLong);
rust_type_impl!(JChar);
rust_type_impl!(JFloat);
rust_type_impl!(JDouble);
rust_type_impl!(JBoolean);



macro_rules! enumification {
    ($v:ty, $out:expr) => {
        impl From<$v> for JavaTypes {
            fn from(value: $v) -> Self {
                $out
            }
        }
    };
}

enumification!(JByte, JavaTypes::Byte);
enumification!(JShort, JavaTypes::Short);
enumification!(JInt, JavaTypes::Int);
enumification!(JLong, JavaTypes::Long);
enumification!(JChar, JavaTypes::Char);
enumification!(JFloat, JavaTypes::Float);
enumification!(JDouble, JavaTypes::Double);
enumification!(JBoolean, JavaTypes::Boolean);




pub trait PrimitiveType: Copy {
    fn get_type() -> JavaTypes;
}

macro_rules! primitivication {
    ($v:ty, $out:expr) => {
        impl PrimitiveType for $v {
            fn get_type() -> JavaTypes {
                $out
            }
        }
    };
}

primitivication!(JByte, JavaTypes::Byte);
primitivication!(JShort, JavaTypes::Short);
primitivication!(JInt, JavaTypes::Int);
primitivication!(JLong, JavaTypes::Long);
primitivication!(JChar, JavaTypes::Char);
primitivication!(JFloat, JavaTypes::Float);
primitivication!(JDouble, JavaTypes::Double);
primitivication!(JBoolean, JavaTypes::Boolean);


#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum JavaTypes {
    Byte,
    Short,
    Int,
    Long,
    Char,
    Float,
    Double,
    Boolean,
    Object,
}
pub const GC_PTR_SIZE: usize = std::mem::size_of::<GcPtr<()>>();
pub const GC_PTR_ALIGN: usize = std::mem::align_of::<GcPtr<()>>();
impl JavaType for JavaTypes {
    fn size(&self) -> usize {
        match self {
            JavaTypes::Byte => std::mem::size_of::<JByte>(),
            JavaTypes::Short => std::mem::size_of::<JShort>(),
            JavaTypes::Int => std::mem::size_of::<JInt>(),
            JavaTypes::Long => std::mem::size_of::<JLong>(),
            JavaTypes::Char => std::mem::size_of::<JChar>(),
            JavaTypes::Float => std::mem::size_of::<JFloat>(),
            JavaTypes::Double => std::mem::size_of::<JDouble>(),
            JavaTypes::Boolean => std::mem::size_of::<JBoolean>(),
            JavaTypes::Object => GC_PTR_SIZE,
        }
    }

    fn align(&self) -> std::num::NonZeroUsize {
        std::num::NonZeroUsize::new(match self {
            JavaTypes::Byte => std::mem::align_of::<JByte>(),
            JavaTypes::Short => std::mem::align_of::<JShort>(),
            JavaTypes::Int => std::mem::align_of::<JInt>(),
            JavaTypes::Long => std::mem::align_of::<JLong>(),
            JavaTypes::Char => std::mem::align_of::<JChar>(),
            JavaTypes::Float => std::mem::align_of::<JFloat>(),
            JavaTypes::Double => std::mem::align_of::<JDouble>(),
            JavaTypes::Boolean => std::mem::align_of::<JBoolean>(),
            JavaTypes::Object => GC_PTR_ALIGN,
        })
        .unwrap()
    }
}

impl From<FieldType> for JavaTypes {
    fn from(value: FieldType) -> Self {
        match value {
            FieldType::ArrayType(_) | FieldType::ObjectType(_) => Self::Object,
            FieldType::BaseType(v) => match v {
                BaseType::Boolean => Self::Boolean,
                BaseType::Byte => Self::Byte,
                BaseType::Char => Self::Char,
                BaseType::Double => Self::Double,
                BaseType::Float => Self::Float,
                BaseType::Int => Self::Int,
                BaseType::Long => Self::Long,
                BaseType::Short => Self::Short
            }
        }
    }
}

impl AsRef<JavaTypes> for FieldType {
    fn as_ref<'a>(&'a self) -> &'a JavaTypes {
        match self {
            FieldType::ArrayType(_) | FieldType::ObjectType(_) => &JavaTypes::Object,
            FieldType::BaseType(v) => match v {
                BaseType::Boolean => &JavaTypes::Boolean,
                BaseType::Byte => &JavaTypes::Byte,
                BaseType::Char => &JavaTypes::Char,
                BaseType::Double => &JavaTypes::Double,
                BaseType::Float => &JavaTypes::Float,
                BaseType::Int => &JavaTypes::Int,
                BaseType::Long => &JavaTypes::Long,
                BaseType::Short => &JavaTypes::Short
            }
        }
    }
}


impl AsRef<JavaTypes> for ExactJavaType {
    fn as_ref(&self) -> &JavaTypes {
        match self {
            Self::Byte => &JavaTypes::Byte,
            ExactJavaType::Short => &JavaTypes::Short,
            ExactJavaType::Int => &JavaTypes::Int,
            ExactJavaType::Long => &JavaTypes::Long,
            ExactJavaType::Char => &JavaTypes::Char,
            ExactJavaType::Float => &JavaTypes::Float,
            ExactJavaType::Double => &JavaTypes::Double,
            ExactJavaType::Boolean => &JavaTypes::Boolean,
            ExactJavaType::Array(_) => &JavaTypes::Object,
            ExactJavaType::ClassInstance(_) => &JavaTypes::Object,
        }
    }
}

impl From<ExactJavaType> for JavaTypes {
    fn from(value: ExactJavaType) -> Self {
        *value.as_ref()
    }
}






#[derive(Clone, Copy)]
pub enum ExactJavaType {
    Byte,
    Short,
    Int,
    Long,
    Char,
    Float,
    Double,
    Boolean,
    Array(GcPtr<ExactJavaType>),
    ClassInstance(GcPtr<Structure>)
}

impl From<BaseType> for ExactJavaType {

    fn from(value: BaseType) -> Self {
        match value {
            BaseType::Boolean => Self::Boolean,
            BaseType::Byte => Self::Byte,
            BaseType::Char => Self::Char,
            BaseType::Double => Self::Double,
            BaseType::Float => Self::Float,
            BaseType::Int => Self::Int,
            BaseType::Long => Self::Long,
            BaseType::Short => Self::Short
        }
    }
}

impl Trace<TheGc> for ExactJavaType {
    fn trace(&mut self, gc: &crate::nugc::collector::GarbageCollector<TheGc>, visitor: &mut <TheGc as crate::nugc::collector::MemoryManager>::VisitorTy) {
        match self {
            Self::Array(v) => visitor.visit(gc, v),
            Self::ClassInstance(v) => visitor.visit(gc, v),
            _ => ()
        }
    }
}


/// Field name and descriptor.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FieldNameAndType {
    pub name: UnqualifiedName,
    pub descriptor: FieldDescriptor,
}



/// Method name and descriptor.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MethodNameAndType {
    pub name: MethodName,
    pub descriptor: MethodDescriptor,
}