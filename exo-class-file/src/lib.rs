//! Class file parser.
//! 
//! Loads class files into an easily usable data structure.


pub mod item;
pub mod stream;
pub mod error;

pub use exo_parser;


// pub struct Object {
//     pub identity_hashcode: JInt,
//     pub has_made_hashcode: bool
// }

// #[java_class_impl("java.lang.Object")]
// impl Object {

//     #[java_native_constructor("")]
//     fn constructor(jvm: JvmRef, this_object: JObject) -> JResult<Self> {
//         Ok(Self {
//             identity_hashcode: 0,
//             has_made_hashcode: false
//         })
//     }

//     #[java_native_method("hashCode", "()I")]
//     fn hashcode(&mut self, jvm: JvmRef, this_object: JObject) -> JResult<JInt> {
//         Ok(self.calc_hashcode(this_object))
//     }
 

//     pub fn calc_hashcode(&mut self, jvm: JvmRef, this_object: JObject) -> JInt {
//         if self.has_made_hashcode {
//             self.identity_hashcode
//         } else {
//             self.identity_hashcode = rand();
//             self.has_made_hashcode = true;
//             self.identity_hashcode
//         }
//     }
// }

// pub struct System {
//     // ...
// }

// #[java_class_impl("java.lang.System")]
// impl System {

//     #[java_native_constructor("")]
//     fn constructor(jvm: JvmRef, this_object: JObject) -> JResult<Self> {
//         Ok(Self {
//             // ...
//         })
//     }

//     #[java_native_method("identityHashCode", "(Ljava/lang/Object;)I")]
//     fn hashcode(&mut self, jvm: JvmRef, this_object: JObject) -> JResult<JInt> {
//         Ok(this_object.native::<Object>(jvm).calc_hashcode(jvm, this_object))
//     }
 
// }
