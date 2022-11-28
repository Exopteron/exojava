use std::{cell::RefCell, char::REPLACEMENT_CHARACTER, num::NonZeroUsize, path::PathBuf, string};

use ahash::AHashMap;
use exo_class_file::{
    exo_parser::tokenimpl::Char,
    item::{
        fields::FieldAccessFlags,
        file::ClassAccessFlags,
        ids::{
            class::{ClassName, ClassRefName},
            field::{ArrayType, BaseType, FieldType, ObjectType},
            method::{MethodDescriptor, MethodName, ReturnDescriptor},
            UnqualifiedName,
        },
        methods::MethodAccessFlags,
    },
};

use crate::memory::{ArrayInitializer, GarbageCollector, Trace};

use self::{
    class::{
        bootstrap::{BootstrapClassLoader, JVMRawClass},
        constant_pool::RuntimeConstantPool,
        FieldNameAndType, JVMError, JvmResult, MethodImplementation, MethodImplementationType,
        MethodNameAndType,
    },
    object::{
        JVMArrayReference, JVMArrayType, JVMClassInstanceTypes, JVMObjectReference,
        JVMRefObjectType, JVMValue, JavaClassInstance,
    },
    thread::JVMThread,
};

pub mod class;
pub mod object;
pub mod thread;

/// The Java Virtual Machine's state.
pub struct JavaVMState {
    /// The object manager/garbage collector.
    /// Handles memory management.
    gc: RefCell<GarbageCollector<Jvm>>,

    /// The bootstrap class loader.
    bootstrap_loader: RefCell<BootstrapClassLoader>,

    running_threads: RefCell<Vec<GcPtr<JVMThread>>>,
}

pub type GcPtr<T> = crate::memory::GcPtr<T, Jvm>;

pub struct Jvm(RefCell<JavaVMState>);

impl Jvm {
    pub fn cool(&self) {
        println!("G");
    }

    /// Set up a JVM.
    pub fn setup(heap_size: NonZeroUsize, classpath: PathBuf) -> Jvm {
        Self::init(Self(RefCell::new(JavaVMState {
            gc: RefCell::new(GarbageCollector::new(heap_size)),
            bootstrap_loader: RefCell::new(BootstrapClassLoader::new(classpath)),
            running_threads: RefCell::new(vec![]),
        })))
    }

    /// JVM initialization
    fn init(self) -> Jvm {
        {
            let jvm = self.0.borrow();
            let mut bootstrap = jvm.bootstrap_loader.borrow_mut();
            let object_class_name = ClassName {
                package: vec!["java".to_string(), "lang".to_string()],
                class_name: "Object".to_string(),
                inner_class: None,
            };

            let class_name = ClassName {
                package: vec!["java".to_string(), "lang".to_string()],
                class_name: "Class".to_string(),
                inner_class: None,
            };

            let d = MethodNameAndType {
                name: MethodName::Init,
                descriptor: MethodDescriptor {
                    parameters: vec![],
                    return_desc: ReturnDescriptor::Void(Char),
                },
            };
            let def = unsafe {
                jvm.gc.borrow_mut().new_object(
                    MethodImplementation {
                        desc: d.clone(),
                        access: MethodAccessFlags::ACC_PUBLIC,
                        imp: MethodImplementationType::Native(|jvm, value| Ok(None)),
                    },
                    None,
                )
            };

            let object_class = bootstrap.define_class_raw(
                &self,
                ClassRefName::Class(object_class_name.clone()),
                JVMRawClass::new(
                    ClassRefName::Class(object_class_name),
                    None,
                    ClassAccessFlags::ACC_PUBLIC,
                    vec![],
                    vec![(d, def)],
                    AHashMap::new(),
                    RuntimeConstantPool::new(),
                ),
            );

            let _class_class = bootstrap.define_class_raw(
                &self,
                ClassRefName::Class(class_name.clone()),
                JVMRawClass::new(
                    ClassRefName::Class(class_name),
                    Some(object_class),
                    ClassAccessFlags::ACC_PUBLIC | ClassAccessFlags::ACC_FINAL,
                    vec![],
                    vec![],
                    AHashMap::new(),
                    RuntimeConstantPool::new(),
                ),
            );

            let string_class_name = ClassName {
                package: vec!["java".to_string(), "lang".to_string()],
                class_name: "String".to_string(),
                inner_class: None,
            };

            let buf_ty = FieldNameAndType {
                name: UnqualifiedName("buf".to_string()),
                descriptor: FieldType::ArrayType(ArrayType(Box::new(FieldType::BaseType(
                    BaseType::Char,
                )))),
            };

            let string_constructor_desc = MethodNameAndType {
                name: MethodName::Init,
                descriptor: MethodDescriptor {
                    parameters: vec![FieldType::ArrayType(ArrayType(Box::new(
                        FieldType::BaseType(BaseType::Char),
                    )))],
                    return_desc: ReturnDescriptor::Void(Char),
                },
            };

            let string_constructor = unsafe {
                jvm.gc.borrow_mut().new_object(
                    MethodImplementation {
                        desc: string_constructor_desc.clone(),
                        access: MethodAccessFlags::ACC_PUBLIC,
                        imp: MethodImplementationType::Native(move |jvm, value| {
                            let this = value[0];
                            let char_array = value[1];
                            if matches!(char_array, JVMValue::Reference(JVMRefObjectType::Null)) {
                                panic!("NPE");
                            }
                            jvm.set_field(
                                this,
                                &FieldNameAndType {
                                    name: UnqualifiedName("buf".to_string()),
                                    descriptor: FieldType::ArrayType(ArrayType(Box::new(
                                        FieldType::BaseType(BaseType::Char),
                                    ))),
                                },
                                char_array,
                            )?;
                            let f = jvm.get_field(
                                this,
                                &FieldNameAndType {
                                    name: UnqualifiedName("buf".to_string()),
                                    descriptor: FieldType::ArrayType(ArrayType(Box::new(
                                        FieldType::BaseType(BaseType::Char),
                                    ))),
                                },
                            )?;;
                            //panic!("F {:?}", f);
                            Ok(None)
                        }),
                    },
                    None,
                )
            };

            let _string_class = bootstrap.define_class_raw(
                &self,
                ClassRefName::Class(string_class_name.clone()),
                JVMRawClass::new(
                    ClassRefName::Class(string_class_name.clone()),
                    Some(object_class),
                    ClassAccessFlags::ACC_PUBLIC | ClassAccessFlags::ACC_FINAL,
                    vec![(FieldAccessFlags::ACC_PRIVATE, buf_ty)],
                    vec![(
                        MethodNameAndType {
                            name: MethodName::Init,
                            descriptor: string_constructor_desc.descriptor,
                        },
                        string_constructor,
                    )],
                    AHashMap::new(),
                    RuntimeConstantPool::new(),
                ),
            );

            let exo_sys_class_name = ClassName {
                package: vec!["com".to_string(), "exopteron".to_string()],
                class_name: "Sys".to_string(),
                inner_class: None,
            };

            let exo_sys_println_desc = MethodNameAndType {
                name: MethodName::Generic(UnqualifiedName("println".to_string())),
                descriptor: MethodDescriptor {
                    parameters: vec![FieldType::ArrayType(ArrayType(Box::new(
                        FieldType::ObjectType(ObjectType {
                            class_name: string_class_name.clone(),
                        }),
                    )))],
                    return_desc: ReturnDescriptor::Void(Char),
                },
            };

            let exo_println = unsafe {
                jvm.gc.borrow_mut().new_object(
                    MethodImplementation {
                        desc: exo_sys_println_desc.clone(),
                        access: MethodAccessFlags::ACC_PUBLIC,
                        imp: MethodImplementationType::Native(move |jvm, value| {
                            let strings = value[0];
                            //println!("Sus");
                            if let JVMValue::Reference(JVMRefObjectType::Array(strings)) = strings {
                                for &elem in { strings.array_ptr.get_ref_slice() } {
                                    //println!("Sex");
                                    print!("{}", jvm.to_rust_string(elem)?);
                                    //println!("Done");
                                }
                                println!();
                            }
                            Ok(None)
                        }),
                    },
                    None,
                )
            };

            let _exo_sys_class = bootstrap.define_class_raw(
                &self,
                ClassRefName::Class(exo_sys_class_name.clone()),
                JVMRawClass::new(
                    ClassRefName::Class(exo_sys_class_name),
                    Some(object_class),
                    ClassAccessFlags::ACC_PUBLIC | ClassAccessFlags::ACC_FINAL,
                    vec![],
                    vec![(
                        MethodNameAndType {
                            name: MethodName::Generic(UnqualifiedName("println".to_string())),
                            descriptor: exo_sys_println_desc.descriptor,
                        },
                        exo_println,
                    )],
                    AHashMap::new(),
                    RuntimeConstantPool::new(),
                ),
            );
        }

        self
    }

    /// Garbage collection sweep
    pub unsafe fn sweep(&self) {
        self.0.borrow().gc.borrow_mut().sweep(&self);
    }

    /// Loads a class with the bootstrap loader.
    pub fn load_class(&self, class_name: ClassRefName) -> JvmResult<GcPtr<JVMRawClass>> {
        println!("Loading");
        let v = self
            .0
            .borrow()
            .bootstrap_loader
            .borrow()
            .load_class(self, class_name.clone());
        println!("Returning {:?}", class_name);
        v
    }

    /// Check if two values are equal
    pub fn equals(&self, object_a: JVMValue, object_b: JVMValue) -> bool {
        if std::mem::discriminant(&object_a) != std::mem::discriminant(&object_b) {
            return false;
        }
        if let JVMValue::Int(a) = object_a {
            if let JVMValue::Int(b) = object_b {
                return a == b;
            }
        }
        if let JVMValue::Reference(a) = object_a {
            if let JVMValue::Reference(b) = object_b {
                if std::mem::discriminant(&a) != std::mem::discriminant(&b) {
                    return false;
                }
                if let JVMRefObjectType::Class(a) = a {
                    if let JVMRefObjectType::Class(b) = b {
                        return a.equals(b);
                    }
                }
                if let JVMRefObjectType::Null = a {
                    return true;
                }
            }
        }
        false
    }

    /// Find method checking supers.
    pub fn find_method_supers(
        &self,
        method: &MethodNameAndType,
        class: GcPtr<JVMRawClass>,
    ) -> JvmResult<GcPtr<MethodImplementation>> {
        let mut c = class;
        println!("Statrting");
        loop {
            if let Ok(m) = self.find_method(method, c) {
                println!("Returning");
                return Ok(m);
            } else if let Some(newc) = unsafe { c.get_ref(0) }.superclass {
                println!("Supared");
                c = newc;
            } else {
                println!("ERrored");
                let throwable_class = self.load_class(ClassRefName::Class(ClassName {
                    package: vec!["java".to_string(), "lang".to_string()],
                    class_name: "Throwable".to_string(),
                    inner_class: None,
                }))?;
                let ex = self.blank_class_instance(throwable_class)?;
                let m = self.find_method(
                    &MethodNameAndType {
                        name: MethodName::Init,
                        descriptor: MethodDescriptor {
                            parameters: vec![],
                            return_desc: ReturnDescriptor::Void(Char),
                        },
                    },
                    throwable_class,
                )?;
                println!("Start");
                self.invoke(m, throwable_class, &[ex])?;
                println!("ENdEERa {:?}", method);
                return Err(JVMError::Exception(ex));
            }
        }
    }

    /// Find a method on a class.
    pub fn find_method(
        &self,
        method: &MethodNameAndType,
        mut class: GcPtr<JVMRawClass>,
    ) -> JvmResult<GcPtr<MethodImplementation>> {
        if let Some(m) = unsafe { class.get_ref(0) }.methods.get(method) {
            Ok(*m)
        } else {
            println!("ERRoRign or {:?}", method);
            let throwable_class = self.load_class(ClassRefName::Class(ClassName {
                package: vec!["java".to_string(), "lang".to_string()],
                class_name: "Throwable".to_string(),
                inner_class: None,
            }))?;
            let ex = self.blank_class_instance(throwable_class)?;
            let m = self.find_method(
                &MethodNameAndType {
                    name: MethodName::Init,
                    descriptor: MethodDescriptor {
                        parameters: vec![],
                        return_desc: ReturnDescriptor::Void(Char),
                    },
                },
                throwable_class,
            )?;
            println!("Start");
            self.invoke(m, throwable_class, &[ex])?;
            println!("ENdEERa {:?}", method);
            Err(JVMError::Exception(ex))
        }
    }

    /// Check if a value is of a type.
    pub fn is_type(&self, ty: &FieldType, val: JVMValue) -> bool {
        match (ty, val) {
            (FieldType::BaseType(BaseType::Int), JVMValue::Int(_)) => true,
            (FieldType::ObjectType(_), JVMValue::Reference(_)) => true,
            (FieldType::ArrayType(_), JVMValue::Reference(JVMRefObjectType::Array(_))) => true,
            _ => false,
        }
    }

    /// TODO implement
    pub fn is_subclass(&self, c: GcPtr<JVMRawClass>, superclass: GcPtr<JVMRawClass>) -> bool {
        true
    }

    pub fn to_rust_string(&self, v: JVMValue) -> JvmResult<String> {
        if let JVMValue::Reference(JVMRefObjectType::Class(_)) = v {
            println!("it is ");
            // panic!(
            //     "Iks:"
            // );
            // todo make sure it is a string
            let buf_ty = FieldNameAndType {
                name: UnqualifiedName("buf".to_string()),
                descriptor: FieldType::ArrayType(ArrayType(Box::new(FieldType::BaseType(
                    BaseType::Char,
                )))),
            };
            let buf = self.get_field(v, &buf_ty)?;
            if let JVMValue::Reference(JVMRefObjectType::Array(buf)) = buf {
                println!("it is buf");
                if matches!(buf.array_type, JVMArrayType::Char) {
                    let rust = unsafe { buf.array_ptr.get_ref_slice() }
                        .iter()
                        .map(|v| {
                            if let JVMValue::Char(v) = v {
                                char::from_u32(*v).unwrap_or(REPLACEMENT_CHARACTER)
                            } else {
                                panic!("balls")
                            }
                        })
                        .collect::<String>();
                    return Ok(rust);
                }
            } else {
                println!("not buf {:?}", buf);
            }
        } else {
            println!("it isnt");
        }
        panic!("Not right")
    }

    pub fn new_string(&self, st: &str) -> JvmResult<JVMValue> {
        let string_class = self.load_class(ClassRefName::Class(ClassName {
            package: vec!["java".to_string(), "lang".to_string()],
            class_name: "String".to_string(),
            inner_class: None,
        }))?;
        let s = self.blank_class_instance(string_class)?;
        let string_constructor_desc = MethodNameAndType {
            name: MethodName::Init,
            descriptor: MethodDescriptor {
                parameters: vec![FieldType::ArrayType(ArrayType(Box::new(
                    FieldType::BaseType(BaseType::Char),
                )))],
                return_desc: ReturnDescriptor::Void(Char),
            },
        };
        let constructor = self.find_method(&string_constructor_desc, string_class)?;

        let st_iter: Vec<JVMValue> = st.chars().map(|v| v as u32).map(JVMValue::Char).collect();

        let array = self.array_instance(
            JVMArrayType::Char,
            st.chars().count(),
            Some(&ArrayInitializer::Values(&st_iter)),
        )?;

        self.invoke(constructor, string_class, &[s, array])?;
        Ok(s)
    }

    /// Set static field of class
    pub fn set_static_field(
        &self,
        field: &FieldNameAndType,
        mut class: GcPtr<JVMRawClass>,
        value: JVMValue,
    ) -> JvmResult<()> {
        let cls = unsafe { class.get(0) };
        if !self.is_type(&field.descriptor, value) {
            panic!("Exception soon");
        }
        if let Some(f) = cls.static_field_values.get_mut(field) {
            *f = value;
        } else {
            panic!("Exception soon");
        }
        Ok(())
    }

    /// Get static field of class
    pub fn get_static_field(
        &self,
        field: &FieldNameAndType,
        mut class: GcPtr<JVMRawClass>,
    ) -> JvmResult<JVMValue> {
        let cls = unsafe { class.get_ref(0) };
        if let Some(f) = cls.static_field_values.get(field) {
            Ok(*f)
        } else {
            panic!("Exception soon");
        }
    }

    /// Invoke a method on a class.
    pub fn invoke(
        &self,
        method: GcPtr<MethodImplementation>,
        mut class: GcPtr<JVMRawClass>,
        arguments: &[JVMValue],
    ) -> JvmResult<Option<JVMValue>> {
        println!("a");
        println!("Hey2");
        let mut thread = {
            let jvm = self.0.borrow();
            let t = unsafe { jvm.gc.borrow_mut().new_object(JVMThread::new(), None) };
            jvm.running_threads.borrow_mut().push(t);
            t
        };
        println!("Hey");
        unsafe {
            println!("RUnningst");
            let v = thread
                .get(0)
                .run_to_completion(self, method, class, arguments);
            self.0.borrow().running_threads.borrow_mut().pop();
            println!("DOne");
            v
        }
    }

    pub fn default_value(&self, v: &FieldType) -> JvmResult<JVMValue> {
        match v {
            FieldType::BaseType(v) => match &v {
                BaseType::Byte => todo!(),
                BaseType::Char => Ok(JVMValue::Char(0)),
                BaseType::Double => todo!(),
                BaseType::Float => todo!(),
                BaseType::Int => Ok(JVMValue::Int(0)),
                BaseType::Long => todo!(),
                BaseType::Short => todo!(),
                BaseType::Boolean => todo!(),
            },
            FieldType::ObjectType(_) => Ok(JVMValue::Reference(JVMRefObjectType::Null)),
            FieldType::ArrayType(_) => Ok(JVMValue::Reference(JVMRefObjectType::Null)),
        }
    }

    /// Creates an array.
    pub fn array_instance(
        &self,
        class: JVMArrayType,
        size: usize,
        init: Option<&ArrayInitializer>,
    ) -> JvmResult<JVMValue> {
        println!("Array size {}", size);
        Ok(unsafe {
            JVMValue::Reference(JVMRefObjectType::Array(JVMArrayReference {
                array_type: class,
                array_ptr: self.0.borrow().gc.borrow_mut().new_array(
                    size,
                    init.unwrap_or(&ArrayInitializer::Value(self.default_value(
                        &<JVMArrayType as std::convert::Into<FieldType>>::into(class),
                    )?)),
                    None,
                ),
            }))
        })
    }

    /// Creates a blank instance of a class. Does not call its constructor.
    pub fn blank_class_instance(&self, mut class: GcPtr<JVMRawClass>) -> JvmResult<JVMValue> {
        let mut fields = AHashMap::new();

        for (flags, field) in &unsafe { class.get_ref(0) }.fields {
            if !flags.contains(FieldAccessFlags::ACC_STATIC) {
                fields.insert(field.clone(), self.default_value(&field.descriptor)?);
            }
        }

        let object = JavaClassInstance { class, fields };
        Ok(unsafe {
            JVMValue::Reference(JVMRefObjectType::Class(JVMObjectReference {
                class: JVMClassInstanceTypes::Java(self
                    .0
                    .borrow()
                    .gc
                    .borrow_mut()
                    .new_object(object, None)),
            }))
        })
    }

    /// Set field of object
    pub fn set_field(
        &self,
        object: JVMValue,
        field: &FieldNameAndType,
        value: JVMValue,
    ) -> JvmResult<()> {
        if let JVMValue::Reference(v) = object {
            if let JVMRefObjectType::Class(v) = v {
                if let JVMClassInstanceTypes::Java(mut v) = unsafe { v.class } {
                    if let Some(f) = unsafe{v.get(0)}.fields.get_mut(field) {
                        if self.is_type(&field.descriptor, value) {
                            *f = value;
                        } else {
                            panic!("Wwrong type {:?} {:?}", field.descriptor, value);
                        }
                    }
                }
            } else {
                // nullpointer exception l8r
            }
        } else {
            // exception l8r
        }
        Ok(())
    }

    /// Get field of object
    pub fn get_field(&self, object: JVMValue, field: &FieldNameAndType) -> JvmResult<JVMValue> {
        if let JVMValue::Reference(v) = object {
            if let JVMRefObjectType::Class(mut v) = v {
                if let JVMClassInstanceTypes::Java(v) = unsafe { v.class } {
                    if let Some(f) = unsafe{v.get_ref(0)}.fields.get(field) {
                        return Ok(*f);
                    }
                }
            } else {
                // nullpointer exception l8r
            }
        } else {
            // exception l8r
        }
        let throwable_class = self.load_class(ClassRefName::Class(ClassName {
            package: vec!["java".to_string(), "lang".to_string()],
            class_name: "Throwable".to_string(),
            inner_class: None,
        }))?;
        let ex = self.blank_class_instance(throwable_class)?;
        let m = self.find_method(
            &MethodNameAndType {
                name: MethodName::Init,
                descriptor: MethodDescriptor {
                    parameters: vec![],
                    return_desc: ReturnDescriptor::Void(Char),
                },
            },
            throwable_class,
        )?;
        self.invoke(m, throwable_class, &[ex])?;
        Err(JVMError::Exception(ex))
    }
    // /// Loads a class.
    // pub fn load_class(&self, class_name: ClassName) -> JvmResult<GcPtr<JVMRawClass>> {
    //     // let jvm = self.0.borrow();

    //     // let mut file = jvm.classpath.clone();
    //     // file.push(format!("{}.class", class_name.class_name));

    //     // self.0.borrow().bootstrap_loader.borrow_mut().load_class(
    //     //     self,
    //     //     class_name,
    //     //     &std::fs::read(file).expect("This will be an exception eventually"),
    //     // )
    // }
}

impl Trace for Jvm {
    unsafe fn trace(&self) {
        let jvm = self.0.borrow();
        jvm.bootstrap_loader.borrow().trace();
        for t in self.0.borrow().running_threads.borrow().iter() {
            t.trace();
        }
    }
}
