// use std::{
//     num::NonZeroUsize,
//     path::{Path, PathBuf},
// };

// use exo_jvm::{
//     exo_class_file::{
//         exo_parser::tokenimpl::Char,
//         item::ids::{
//             class::{ClassName, ClassRefName},
//             field::{BaseType, FieldType, ObjectType},
//             method::{MethodDescriptor, MethodName, ReturnDescriptor},
//             UnqualifiedName,
//         },
//     },
//     memory::Trace,
//     vm::{
//         class::{FieldNameAndType, JVMError, MethodNameAndType},
//         object::{JVMObjectReference, JVMRefObjectType, JVMValue, JVMClassInstanceTypes},
//         Jvm,
//     },
// };

// fn main() {
//     let mut jvm = Jvm::setup(
//         NonZeroUsize::new(1 * 1_000_000).unwrap(),
//         Path::new("classdir").to_path_buf(),
//     );
//     let name = ClassName {
//         package: vec![],
//         class_name: "Epic".to_string(),
//         inner_class: None,
//     };
//     // unsafe {
//     //     jvm.trace();
//     //     jvm.sweep();
//     // }
//     let class = jvm.load_class(ClassRefName::Class(name.clone()));
//     if let Err(c) = class {
//         panic!("Errored");
//     } else if let Ok(mut c) = class {
//         //println!("Class: {:#?}", c);
//         // let method = jvm.find_static_method(MethodNameAndType { name: MethodName::Generic(UnqualifiedName("gamer".to_string())), descriptor: MethodDescriptor {
//         //     parameters: vec![],
//         //     return_desc: ReturnDescriptor::Field(FieldType::BaseType(BaseType::Int)),
//         // } }, c).unwrap();
//         // let v = jvm.invoke_static(method, c, &[]);

//         // for method in &unsafe { c.get_ref() }.methods {
//         //     println!("M: {:?}", method);
//         // }
//         let method = jvm.find_method(
//             &MethodNameAndType {
//                 name: MethodName::Generic(UnqualifiedName("cool".to_string())),
//                 descriptor: MethodDescriptor {
//                     parameters: vec![FieldType::BaseType(BaseType::Int)],
//                     return_desc: ReturnDescriptor::Field(FieldType::BaseType(BaseType::Int)),
//                 },
//             },
//             c,
//         );
//         if let Err(e) = method {
//             panic!("Exception");
//         }
//         let method = method.unwrap();

//         println!("AAASS");
//         match jvm.invoke(method, c, &[JVMValue::Int(420)]) {
//             Ok(v) => println!("Value: {:?}", v),
//             Err(_) => todo!(),
//         }
//         // let field = jvm.get_field(
//         //     object,
//         //     &FieldNameAndType {
//         //         name: UnqualifiedName("x".to_string()),
//         //         descriptor: FieldType::BaseType(BaseType::Int),
//         //     },
//         // ).unwrap();
//         // println!("Vale: {:?}", field);

//         // let value = jvm.get_static_field(
//         //     &FieldNameAndType {
//         //         name: UnqualifiedName("x".to_string()),
//         //         descriptor: FieldType::BaseType(BaseType::Int),
//         //     },
//         //     c,
//         // );
//         // println!("Value: {:?}", value);
//     }
//     let jvm = jvm.cool();

//     println!("G");
// }
fn main() {
    
}