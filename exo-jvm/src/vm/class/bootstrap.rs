use std::{collections::HashMap, fmt::Debug, io::Cursor, path::PathBuf, cell::RefCell};

use ahash::AHashMap;
use exo_class_file::{
    exo_parser::{tokenimpl::Char, Lexer},
    item::{
        attribute_info::{attrtype, Attributes},
        constant_pool::ConstantPoolEntry,
        fields::FieldAccessFlags,
        file::{ClassAccessFlags, ClassFile},
        ids::{
            class::{ClassName, ClassRefName},
            field::{BaseType, FieldDescriptor, FieldType},
            method::{MethodDescriptor, MethodName, ReturnDescriptor},
            UnqualifiedName,
        },
        methods::MethodAccessFlags,
        ClassFileItem,
    },
    stream::ClassFileStream,
};

use crate::{memory::Trace, vm::object::JVMClassInstanceTypes};

use super::{
    super::{
        object::{JVMRefObjectType, JVMValue},
        GcPtr, Jvm,
    },
    constant_pool::{ConstantClassInfo, ConstantFieldRef, RuntimeConstant, RuntimeConstantPool, ConstantMethodRef, ConstantStringRef},
    FieldNameAndType, JavaExceptionTableEntry, JavaMethodCode, JvmResult, MethodImplementation,
    MethodImplementationType, MethodNameAndType,
};

/// Bootstrap class loader.
pub struct BootstrapClassLoader {
    classes: RefCell<AHashMap<ClassRefName, GcPtr<JVMRawClass>>>,
    class_path: PathBuf,
}

macro_rules! parse_str {
    ($str:expr, $token:ty) => {{
        let lexer = Lexer::new();
        let mut stream = Lexer::stream(lexer, $str);
        stream.token::<$token>().expect("exception eventually")
    }};
}

macro_rules! get_class {
    ($index:expr, $cp:expr, $parsety:ty) => {{
        if let ConstantPoolEntry::Class { name_index } = $cp.get_constant($index as usize) {
            let str = $cp
                .get_utf8_constant(*name_index as usize)
                .expect("Exception in the future");
            parse_str!(str.to_string(), $parsety)
        } else {
            panic!("Exception eventually")
        }
    }};
}

macro_rules! load_class {
    ($cl:expr, $jvm:expr, $cp:expr, $index:expr) => {{
        let name = get_class!($index, $cp, ClassRefName);
        $cl.load_class($jvm, name.token)
    }};
}

macro_rules! get_constant {
    ($g:tt, $index:expr, $cp:expr, $($p:tt),*) => {
        if let ConstantPoolEntry::$g { $($p),* } = $cp.get_constant($index as usize) {
            ($($p),*)
        } else {
            panic!("TODO exception soon")
        }
    };
}

macro_rules! get_attr {
    ($g:tt, $v:expr, $($p:tt),*) => {
        if let Attributes::$g { $($p),* } = $v {
            ($($p),*)
        } else {
            panic!("TODO exception soon")
        }
    };
}

impl BootstrapClassLoader {
    pub fn new(class_path: PathBuf) -> Self {
        Self {
            classes: Default::default(),
            class_path,
        }
    }

    /// Finds a class's bytes.
    pub fn find_class(&self, class_name: ClassName) -> JvmResult<Vec<u8>> {
        let mut file = self.class_path.clone();
        file.push(format!("{}.class", class_name.class_name));

        Ok(std::fs::read(file).expect("This will be an exception eventually"))
    }

    pub fn load_array_class(&self, jvm: &Jvm, element_type: FieldDescriptor) -> JvmResult<GcPtr<JVMRawClass>> {
        let jvm_class = JVMRawClass::new(
            ClassRefName::Array(element_type.clone()),
            None,
            ClassAccessFlags::ACC_PUBLIC,
            vec![],
            vec![],
            AHashMap::new(),
            RuntimeConstantPool { pool: vec![] },
        );
        Ok(self.define_class_raw(jvm, ClassRefName::Array(element_type), jvm_class))
    }

    /// Loads a class.
    pub fn load_class(&self, jvm: &Jvm, name: ClassRefName) -> JvmResult<GcPtr<JVMRawClass>> {
        if let Some(c) = self.classes.borrow().get(&name) {
            return Ok(*c);
        }
        let name = if let ClassRefName::Class(c) = name {
            c
        } else {
            if let ClassRefName::Array(c) = name {
                return self.load_array_class(jvm, c);
            }
            unreachable!()
        };
        let bytes = self.find_class(name.clone())?;
        let mut c = Cursor::new(bytes);
        let mut reader = ClassFileStream::new(&mut c);

        let mut class = ClassFile::read_from_stream(&mut reader, None)
            .expect("this will be an exception in the future");
        let mut cl = {
            let class_info = get_class!(class.this_class, class.constant_pool, ClassRefName);
            if class_info.token != ClassRefName::Class(name.clone()) {
                panic!("exception eventually - class name mismatch");
            }

            let superclass_name = get_class!(class.super_class, class.constant_pool, ClassRefName);
            let superclass = self.load_class(jvm, superclass_name.token)?;

            let mut static_field_values = AHashMap::new();

            let mut runtime_constant_pool = RuntimeConstantPool::new();

            let mut fields = vec![];
            let mut methods = vec![];
            let jvm_class = JVMRawClass::new(
                ClassRefName::Class(name.clone()),
                Some(superclass),
                class.access_flags,
                fields,
                methods,
                static_field_values,
                runtime_constant_pool,
            );
            self.define_class_raw(jvm, ClassRefName::Class(name), jvm_class)
        };

        {
            self.load_class_fields(jvm, &mut class, cl)?;
            self.load_class_methods(jvm, &mut class, cl)?;

            self.load_runtime_constant_pool(jvm, &class, cl)?;
        }

        println!("BEginginging");
        if let Ok(instance_initializer) = jvm.find_method(
            &MethodNameAndType {
                name: MethodName::Clinit,
                descriptor: MethodDescriptor {
                    parameters: vec![],
                    return_desc: ReturnDescriptor::Void(Char),
                },
            },
            cl,
        ) {
            jvm.invoke(instance_initializer, cl, &[])?;
        }
        println!("DUnn");

        Ok(cl)
    }

    fn load_class_methods(
        &self,
        jvm: &Jvm,
        class_file: &mut ClassFile,
        mut class: GcPtr<JVMRawClass>,
    ) -> JvmResult<()> {
        for mut method in class_file.methods.drain(..) {
            let name = parse_str!(
                class_file
                    .constant_pool
                    .get_utf8_constant(method.name_index as usize)
                    .expect("exception eventually")
                    .to_string(),
                MethodName
            );
            let descriptor = parse_str!(
                class_file
                    .constant_pool
                    .get_utf8_constant(method.descriptor_index as usize)
                    .expect("exception eventually")
                    .to_string(),
                MethodDescriptor
            );

            let ty = MethodNameAndType {
                name: name.token,
                descriptor: descriptor.token,
            };

            let code = method.attributes.take(attrtype::Code).swap_remove(0);

            let mut real_declared_exceptions = vec![];

            let mut declared_exceptions = method.attributes.take(attrtype::Exceptions);
            if !declared_exceptions.is_empty() {
                if let Attributes::Exceptions {
                    exception_index_table,
                } = declared_exceptions.swap_remove(0)
                {
                    for v in exception_index_table {
                        let class = get_class!(v, class_file.constant_pool, ClassRefName);
                        real_declared_exceptions.push(self.load_class(jvm, class.token)?);
                    }
                }
            }

            if let Attributes::Code {
                max_stack,
                max_locals,
                code,
                exception_table,
                attributes,
            } = code
            {
                let mut real_exception_table = Vec::with_capacity(exception_table.len());
                for v in exception_table {
                    real_exception_table.push(JavaExceptionTableEntry {
                        pc_range: (*v.pc_range.start(), *v.pc_range.end()),
                        handler_pc: v.handler_pc,
                        catch_type: load_class!(self, jvm, class_file.constant_pool, v.catch_type)?,
                    });
                }
                unsafe {
                    class.get(0).methods.insert(
                        ty.clone(),
                        jvm.0.borrow().gc.borrow_mut().new_object(
                            MethodImplementation::new(
                                ty,
                                method.access_flags,
                                MethodImplementationType::Java {
                                    code: JavaMethodCode {
                                        max_stack,
                                        max_locals,
                                        code,
                                        exception_table: real_exception_table,
                                        attributes,
                                    },
                                    declared_exceptions: real_declared_exceptions,
                                },
                            ),
                            None,
                        ),
                    );
                }
            }
        }
        Ok(())
    }

    fn load_class_fields(
        &self,
        jvm: &Jvm,
        class_file: &mut ClassFile,
        mut class: GcPtr<JVMRawClass>,
    ) -> JvmResult<()> {
        for mut field in class_file.fields.drain(..) {
            let name = parse_str!(
                get_constant!(Utf8, field.name_index, class_file.constant_pool, data).to_owned(),
                UnqualifiedName
            );
            let descriptor = parse_str!(
                get_constant!(Utf8, field.descriptor_index, class_file.constant_pool, data)
                    .to_owned(),
                FieldDescriptor
            );
            let fnat = FieldNameAndType {
                name: name.token,
                descriptor: descriptor.token,
            };
            if field.access_flags.contains(FieldAccessFlags::ACC_STATIC) {
                let v = jvm.default_value(&fnat.descriptor)?;
                unsafe { class.get(0).static_field_values.insert(fnat.clone(), v) };

                if field.access_flags.contains(FieldAccessFlags::ACC_FINAL) {
                    let mut constants = field.attributes.take(attrtype::ConstantValue);
                    if !constants.is_empty() {
                        let constantvalue_index =
                            get_attr!(ConstantValue, constants.swap_remove(0), constantvalue_index);
                        match class_file
                            .constant_pool
                            .get_constant(constantvalue_index as usize)
                        {
                            ConstantPoolEntry::Integer { bytes } => {
                                unsafe { class.get(0) }
                                    .static_field_values
                                    .insert(fnat.clone(), JVMValue::Int(*bytes));
                            }
                            _ => panic!("Exception eventually"),
                        }
                    }
                }
            }
            unsafe { class.get(0) }
                .fields
                .push((field.access_flags, fnat));
        }
        Ok(())
    }

    /// Loads the run-time constant pool.
    fn load_runtime_constant_pool(
        &self,
        jvm: &Jvm,
        class_file: &ClassFile,
        mut class: GcPtr<JVMRawClass>,
    ) -> JvmResult<()> {
        for entry in class_file.constant_pool.entries.iter() {
            let x = match entry {
                ConstantPoolEntry::String { string_index } => {
                    let data = class_file
                    .constant_pool
                    .get_utf8_constant(*string_index as usize)
                    .expect("Exception in the future");

                    RuntimeConstant::String(ConstantStringRef {
                        value: jvm.new_string(data)?,
                    })
                },
                ConstantPoolEntry::Class { name_index } => {
                    let str = class_file
                        .constant_pool
                        .get_utf8_constant(*name_index as usize)
                        .expect("Exception in the future");
                    let name = parse_str!(str.to_string(), ClassRefName);
                    println!("Loading self");
                    RuntimeConstant::Class(ConstantClassInfo {
                        class: self.load_class(jvm, name.token)?,
                    })
                }
                ConstantPoolEntry::Fieldref {
                    class_index,
                    name_and_type_index,
                } => {
                    let name = parse_str!(
                        get_constant!(
                            Utf8,
                            *get_constant!(
                                Class,
                                *class_index,
                                class_file.constant_pool,
                                name_index
                            ),
                            class_file.constant_pool,
                            data
                        )
                        .to_owned(),
                        ClassRefName
                    )
                    .token;
                    let (name_index, descriptor_index) = get_constant!(
                        NameAndType,
                        *name_and_type_index,
                        class_file.constant_pool,
                        name_index,
                        descriptor_index
                    );

                    let field_name = parse_str!(
                        class_file
                            .constant_pool
                            .get_utf8_constant(*name_index as usize)
                            .expect("exception eventually")
                            .to_string(),
                        UnqualifiedName
                    );
                    let descriptor = parse_str!(
                        class_file
                            .constant_pool
                            .get_utf8_constant(*descriptor_index as usize)
                            .expect("exception eventually")
                            .to_string(),
                        FieldDescriptor
                    );

                    let ty = FieldNameAndType {
                        name: field_name.token,
                        descriptor: descriptor.token,
                    };
                    RuntimeConstant::Field(ConstantFieldRef {
                        class: self.load_class(jvm, name)?,
                        field: ty,
                    })
                },
                ConstantPoolEntry::Methodref { class_index, name_and_type_index } => {
                    let name = parse_str!(
                        get_constant!(
                            Utf8,
                            *get_constant!(
                                Class,
                                *class_index,
                                class_file.constant_pool,
                                name_index
                            ),
                            class_file.constant_pool,
                            data
                        )
                        .to_owned(),
                        ClassRefName
                    )
                    .token;
                    let (name_index, descriptor_index) = get_constant!(
                        NameAndType,
                        *name_and_type_index,
                        class_file.constant_pool,
                        name_index,
                        descriptor_index
                    );

                    let field_name = parse_str!(
                        class_file
                            .constant_pool
                            .get_utf8_constant(*name_index as usize)
                            .expect("exception eventually")
                            .to_string(),
                        MethodName
                    );
                    let descriptor = parse_str!(
                        class_file
                            .constant_pool
                            .get_utf8_constant(*descriptor_index as usize)
                            .expect("exception eventually")
                            .to_string(),
                        MethodDescriptor
                    );

                    let ty = MethodNameAndType {
                        name: field_name.token,
                        descriptor: descriptor.token,
                    };
                    RuntimeConstant::Method(ConstantMethodRef {
                        class: self.load_class(jvm, name)?,
                        method: ty,
                    })
                }
                v => RuntimeConstant::Other(v.clone()),
            };
            unsafe { class.get(0) }.runtime_constant_pool.pool.push(x);
        }
        Ok(())
    }

    /// Defines a raw class.
    pub fn define_class_raw(
        &self,
        jvm: &Jvm,
        name: ClassRefName,
        c: JVMRawClass,
    ) -> GcPtr<JVMRawClass> {
        let gc_ptr = unsafe { jvm.0.borrow().gc.borrow_mut().new_object(c, None) };
        self.classes.borrow_mut().insert(name, gc_ptr);
        gc_ptr
    }
}

impl Trace for BootstrapClassLoader {
    unsafe fn trace(&self) {
        for (_, cl) in self.classes.borrow().iter() {
            cl.trace();
        }
    }
}

/// An instance of `java.lang.Class`.
/// Has special behavior.
#[derive(Debug)]
pub struct JVMRawClass {
    /// The class's name.
    pub name: ClassRefName,

    /// The class's superclass.
    pub superclass: Option<GcPtr<JVMRawClass>>,

    /// The class's access flags.
    pub access: ClassAccessFlags,

    /// The class's fields.
    pub fields: Vec<(FieldAccessFlags, FieldNameAndType)>,

    /// Value of static fields.
    pub static_field_values: AHashMap<FieldNameAndType, JVMValue>,

    /// The class's methods.
    pub methods: AHashMap<MethodNameAndType, GcPtr<MethodImplementation>>,

    pub runtime_constant_pool: RuntimeConstantPool,
}

impl Trace for JVMRawClass {
    unsafe fn trace(&self) {
        if let Some(s) = self.superclass {
            s.trace();
        }
        for (_, v) in self.methods.iter() {
            v.trace();
        }
        for (_, v) in self.static_field_values.iter() {
            v.trace();
        }
    }
}

impl JVMRawClass {
    pub fn new(
        name: ClassRefName,
        superclass: Option<GcPtr<JVMRawClass>>,
        access: ClassAccessFlags,
        fields: Vec<(FieldAccessFlags, FieldNameAndType)>,
        methods: Vec<(MethodNameAndType, GcPtr<MethodImplementation>)>,
        static_field_values: AHashMap<FieldNameAndType, JVMValue>,
        runtime_constant_pool: RuntimeConstantPool,
    ) -> Self {
        let mut map = AHashMap::new();
        for (k, v) in methods {
            map.insert(k, v);
        }
        Self {
            fields,
            superclass,
            methods: map,
            name,
            static_field_values,
            access,
            runtime_constant_pool,
        }
    }

    /// The name.
    pub fn name() -> ClassName {
        ClassName {
            package: vec!["java".to_string(), "lang".to_string()],
            class_name: "Class".to_string(),
            inner_class: None,
        }
    }
}

// /// An instance of a native class.
// pub struct JVMNativeClassInstance {
//     class: JVMRefObjectType,
//     name: ClassRefName,
//     fields: AHashMap<FieldNameAndType, JVMNativeClassField>
// }

// impl JVMNativeClassInstance {
//     /// Get this native class's name.
//     pub fn name(&self) -> &ClassRefName {
//         &self.name
//     }

//     /// Create a new native class.
//     pub fn new(class: JVMRefObjectType, name: ClassRefName, fields: Vec<(FieldNameAndType, JVMNativeClassField)>) -> Self {
//         let mut map = AHashMap::new();
//         for (k, v) in fields {
//             map.insert(k, v);
//         }
//         Self { class, name, fields: map }
//     }

//     pub fn set_class(&mut self, c: JVMRefObjectType) {
//         self.class = c;
//     }
// }

// /// Native class field.
// pub struct JVMNativeClassField {
//     id: FieldNameAndType,
//     value: JVMValue,
//     modifiers: FieldAccessFlags
// }

// /// A Java class.
// pub struct JavaClass {
//     name: ClassRefName,
//     loader: ClassLoaderType
// }

// impl JavaClass {
//     /// Get the binary name of this class.
//     pub fn name(&self) -> &ClassRefName {
//         &self.name
//     }
// }
