use std::{
    io::{Cursor, Read},
    ops::{Range, RangeInclusive}, collections::HashMap,
};

use crate::{
    error::{self, ClassFileError},
    item::{constant_pool::ConstantPool, file::ClassAccessFlags, ClassFileItem},
    stream::ClassFileStream,
};

use self::{
    attrtype::{
        AnnotationDefault, BootstrapMethods, Code, ConstantValue, Deprecated, EnclosingMethod,
        Exceptions, InnerClasses, LineNumberTable, LocalVariableTable, LocalVariableTypeTable,
        MethodParameters, RuntimeInvisibleAnnotations, RuntimeInvisibleParameterAnnotations,
        RuntimeInvisibleTypeAnnotations, RuntimeVisibleAnnotations,
        RuntimeVisibleParameterAnnotations, RuntimeVisibleTypeAnnotations, Signature,
        SourceDebugExtension, SourceFile, StackMapTable, Synthetic,
    },
    elementvaluetypes::ElementValue,
    stackmap::StackMapFrame,
    typepathkinds::TypePathKind,
};

use super::opcodes::InstructionList;

/// Verification type items.
mod verification {
    use std::io::Read;

    use crate::{
        error::{self, ClassFileError},
        item::{constant_pool::ConstantPool, ClassFileItem},
        stream::ClassFileStream,
    };

    pub const ITEM_Top: u8 = 0;
    pub const ITEM_Integer: u8 = 1;
    pub const ITEM_Float: u8 = 2;
    pub const ITEM_Double: u8 = 3;
    pub const ITEM_Long: u8 = 4;
    pub const ITEM_Null: u8 = 5;
    pub const ITEM_UninitializedThis: u8 = 6;
    pub const ITEM_Object: u8 = 7;
    pub const ITEM_Uninitialized: u8 = 8;

    /// Verification types.
    #[derive(Debug)]
    pub enum VerificationTypeInfo {
        /// The Top_variable_info item indicates that the local variable has the verification type top.
        Top,
        /// The Integer_variable_info item indicates that the location has the verification type int.
        Integer,
        /// The Float_variable_info item indicates that the location has the verification type float.
        Float,

        /// The Double_variable_info item indicates that the first of two locations has the verification type double.
        ///
        /// The Long_variable_info and Double_variable_info items indicate the verification type of the second of two locations as follows:
        ///
        /// If the first of the two locations is a local variable, then:
        ///
        /// It must not be the local variable with the highest index.
        ///
        /// The next higher numbered local variable has the verification type top.
        ///
        /// If the first of the two locations is an operand stack entry, then:
        ///
        /// It must not be the topmost location of the operand stack.
        ///
        /// The next location closer to the top of the operand stack has the verification type top.
        Double,
        /// The Long_variable_info item indicates that the first of two locations has the verification type long.
        ///
        /// The Long_variable_info and Double_variable_info items indicate the verification type of the second of two locations as follows:
        ///
        /// If the first of the two locations is a local variable, then:
        ///
        /// It must not be the local variable with the highest index.
        ///
        /// The next higher numbered local variable has the verification type top.
        ///
        /// If the first of the two locations is an operand stack entry, then:
        ///
        /// It must not be the topmost location of the operand stack.
        ///
        /// The next location closer to the top of the operand stack has the verification type top.
        Long,
        /// The Null_variable_info type indicates that the location has the verification type null.
        Null,
        /// The UninitializedThis_variable_info item indicates that the location has the verification type uninitializedThis.
        UninitializedThis,
        /// The Object_variable_info item indicates that the location has the verification type which is the class
        /// represented by the CONSTANT_Class_info structure (§4.4.1) found in the constant_pool table at the index
        /// given by cpool_index.
        Object {
            /// Class index in constant pool.
            cpool_index: u16,
        },
        /// The Uninitialized_variable_info item indicates that the location has the
        /// verification type uninitialized(Offset). The Offset item indicates the offset,
        /// in the code array of the Code attribute that contains this StackMapTable
        /// attribute, of the new instruction (§new) that created the object
        /// being stored in the location.
        Uninitialized { offset: u16 },
    }

    impl ClassFileItem for VerificationTypeInfo {
        fn read_from_stream<R: Read>(
            s: &mut ClassFileStream<R>,
            cp: Option<&ConstantPool>,
        ) -> error::Result<Self>
        where
            Self: Sized,
        {
            match s.read_u1()? {
                ITEM_Top => Ok(Self::Top),
                ITEM_Integer => Ok(Self::Integer),
                ITEM_Float => Ok(Self::Float),
                ITEM_Long => Ok(Self::Long),
                ITEM_Double => Ok(Self::Double),
                ITEM_Null => Ok(Self::Null),
                ITEM_UninitializedThis => Ok(Self::UninitializedThis),
                ITEM_Object => Ok(Self::Object {
                    cpool_index: s.read_u2()?,
                }),
                ITEM_Uninitialized => Ok(Self::Uninitialized {
                    offset: s.read_u2()?,
                }),
                _ => Err(ClassFileError::UnknownVerificationTypeInfo),
            }
        }
    }
}

/// Stack map frame items.
mod stackmap {

    use std::{io::Read, ops::Range};

    use crate::{
        error::{self, ClassFileError},
        item::{constant_pool::ConstantPool, ClassFileItem},
        stream::ClassFileStream,
    };

    use super::verification::VerificationTypeInfo;

    pub const SAME: Range<u8> = (0..63);
    pub const SAME_LOCALS_1_STACK_ITEM: Range<u8> = (64..127);
    pub const SAME_LOCALS_1_STACK_ITEM_EXTENDED: u8 = 247;
    pub const CHOP: Range<u8> = (248..250);
    pub const SAME_FRAME_EXTENDED: u8 = 251;
    pub const APPEND: Range<u8> = (252..254);
    pub const FULL_FRAME: u8 = 255;

    // TODO verify validity
    /// A stack map frame.
    #[derive(Debug)]
    pub enum StackMapFrame {
        /// The frame type same_frame is represented by tags in the range [0-63].
        ///
        /// This frame type indicates that the frame has exactly the same local
        /// variables as the previous frame and that the operand stack is empty.
        ///
        /// The offset_delta value for the frame is the value
        /// of the tag item, frame_type.
        SameFrame,

        /// The frame type same_locals_1_stack_item_frame is represented by tags
        /// in the range [64, 127].
        ///
        /// This frame type indicates that the frame has
        /// exactly the same local variables as the previous frame and that the
        /// operand stack has one entry.
        ///
        /// The offset_delta value for the frame is
        /// given by the formula frame_type - 64. The verification type of the
        /// one stack entry appears after the frame type.
        SameLocals1StackItemFrame { stack: VerificationTypeInfo },
        /// The frame type same_locals_1_stack_item_frame_extended is represented
        /// by the tag 247.
        ///
        /// This frame type indicates that the frame has exactly
        /// the same local variables as the previous frame and that the operand
        /// stack has one entry.
        ///
        /// The offset_delta value for the frame is given
        /// explicitly, unlike in the frame type same_locals_1_stack_item_frame.
        /// The verification type of the one stack entry appears after offset_delta.
        SameLocals1StackItemFrameExtended {
            offset_delta: u16,
            stack: VerificationTypeInfo,
        },
        /// The frame type chop_frame is represented by tags in the range [248-250].
        ///
        /// This frame type indicates that the frame has the same local variables
        /// as the previous frame except that the last k local variables are absent,
        /// and that the operand stack is empty.
        ///
        /// The value of k is given by the formula 251 - frame_type. The offset_delta
        /// value for the frame is given explicitly.
        ChopFrame { offset_delta: u16 },
        /// The frame type same_frame_extended is represented by the tag 251.
        ///
        /// This frame type indicates that the frame has exactly the same
        /// local variables as the previous frame and that the operand stack
        /// is empty.
        ///
        /// The offset_delta value for the frame is given explicitly,
        /// unlike in the frame type same_frame.
        SameFrameExtended { offset_delta: u16 },
        /// The frame type append_frame is represented by tags in the range [252-254].
        ///
        /// This frame type indicates that the frame has the same locals as the
        /// previous frame except that k additional locals are defined, and
        /// that the operand stack is empty.
        ///
        /// The value of k is given by the formula frame_type - 251.
        /// The offset_delta value for the frame is given explicitly.
        AppendFrame {
            offset_delta: u16,
            locals: Vec<VerificationTypeInfo>,
        },
        /**
        The frame type full_frame is represented by the tag 255. The offset_delta value for the frame is given explicitly.
        The 0th entry in locals represents the verification type of local variable 0. If locals[M] represents local variable N, then:

        locals[M+1] represents local variable N+1 if locals[M] is one of Top_variable_info, Integer_variable_info, Float_variable_info, Null_variable_info, UninitializedThis_variable_info, Object_variable_info, or Uninitialized_variable_info; and

        locals[M+1] represents local variable N+2 if locals[M] is either Long_variable_info or Double_variable_info.

        It is an error if, for any index i, locals[i] represents a local variable whose index is greater than the maximum number of local variables for the method.

        The 0th entry in stack represents the verification type of the bottom of the operand stack, and subsequent entries in stack represent the verification types of stack entries closer to the top of the operand stack. We refer to the bottom of the operand stack as stack entry 0, and to subsequent entries of the operand stack as stack entry 1, 2, etc. If stack[M] represents stack entry N, then:

        stack[M+1] represents stack entry N+1 if stack[M] is one of Top_variable_info, Integer_variable_info, Float_variable_info, Null_variable_info, UninitializedThis_variable_info, Object_variable_info, or Uninitialized_variable_info; and

        stack[M+1] represents stack entry N+2 if stack[M] is either Long_variable_info or Double_variable_info.

        It is an error if, for any index i, stack[i] represents a stack entry whose index is greater than the maximum operand stack size for the method.
        **/
        FullFrame {
            offset_delta: u16,
            locals: Vec<VerificationTypeInfo>,
            stack: Vec<VerificationTypeInfo>,
        },
    }
    impl ClassFileItem for StackMapFrame {
        fn read_from_stream<R: Read>(
            s: &mut ClassFileStream<R>,
            cp: Option<&ConstantPool>,
        ) -> error::Result<Self>
        where
            Self: Sized,
        {
            match s.read_u1()? {
                v if SAME.contains(&v) => Ok(Self::SameFrame),
                v if SAME_LOCALS_1_STACK_ITEM.contains(&v) => Ok(Self::SameLocals1StackItemFrame {
                    stack: VerificationTypeInfo::read_from_stream(s, cp)?,
                }),
                SAME_LOCALS_1_STACK_ITEM_EXTENDED => Ok(Self::SameLocals1StackItemFrameExtended {
                    offset_delta: s.read_u2()?,
                    stack: VerificationTypeInfo::read_from_stream(s, cp)?,
                }),
                v if CHOP.contains(&v) => Ok(Self::ChopFrame {
                    offset_delta: s.read_u2()?,
                }),
                SAME_FRAME_EXTENDED => Ok(Self::SameFrameExtended {
                    offset_delta: s.read_u2()?,
                }),
                v if APPEND.contains(&v) => Ok(Self::AppendFrame {
                    offset_delta: s.read_u2()?,
                    locals: s.read_sequence(cp, (v as usize) - 251)?,
                }),
                FULL_FRAME => {
                    let offset_delta = s.read_u2()?;
                    let number_of_locals = s.read_u2()?;
                    let locals = s.read_sequence(cp, number_of_locals as usize)?;
                    let number_of_stack_items = s.read_u2()?;
                    let stack = s.read_sequence(cp, number_of_stack_items as usize)?;
                    Ok(Self::FullFrame {
                        offset_delta,
                        locals,
                        stack,
                    })
                }
                v => Err(ClassFileError::UnknownStackMapFrameTag(v)),
            }
        }
    }
}
/// Attribute types.
pub mod attrtype {
    pub const ConstantValue: &'static str = "ConstantValue";
    pub const Code: &'static str = "Code";
    pub const StackMapTable: &'static str = "StackMapTable";
    pub const Exceptions: &'static str = "Exceptions";
    pub const BootstrapMethods: &'static str = "BootstrapMethods";
    pub const InnerClasses: &'static str = "InnerClasses";
    pub const EnclosingMethod: &'static str = "EnclosingMethod";
    pub const Synthetic: &'static str = "Synthetic";
    pub const Signature: &'static str = "Signature";
    pub const RuntimeVisibleAnnotations: &'static str = "RuntimeVisibleAnnotations";
    pub const RuntimeInvisibleAnnotations: &'static str = "RuntimeInvisibleAnnotations";
    pub const RuntimeVisibleParameterAnnotations: &'static str =
        "RuntimeVisibleParameterAnnotations";
    pub const RuntimeInvisibleParameterAnnotations: &'static str =
        "RuntimeInvisibleParameterAnnotations";
    pub const RuntimeVisibleTypeAnnotations: &'static str = "RuntimeVisibleTypeAnnotations";
    pub const RuntimeInvisibleTypeAnnotations: &'static str = "RuntimeInvisibleTypeAnnotations";
    pub const AnnotationDefault: &'static str = "AnnotationDefault";
    pub const MethodParameters: &'static str = "MethodParameters";
    pub const SourceFile: &'static str = "SourceFile";
    pub const SourceDebugExtension: &'static str = "SourceDebugExtension";
    pub const LineNumberTable: &'static str = "LineNumberTable";
    pub const LocalVariableTable: &'static str = "LocalVariableTable";
    pub const LocalVariableTypeTable: &'static str = "LocalVariableTypeTable";
    pub const Deprecated: &'static str = "Deprecated";
}

/// Attributes in a class file.
///
/// These are used in the `ClassFile`, `field_info`, `method_info`
/// and `Code_attribute` structures of the class file format.
#[derive(Debug)]
pub enum Attributes {
    /**
    The ConstantValue attribute is a fixed-length attribute in the attributes table of a field_info structure (§4.5). A ConstantValue attribute represents the value of a constant expression (JLS §15.28), and is used as follows:

    If the ACC_STATIC flag in the access_flags item of the field_info structure is set, then the field represented by the field_info structure is assigned the value represented by its ConstantValue attribute as part of the initialization of the class or interface declaring the field (§5.5). This occurs prior to the invocation of the class or interface initialization method of that class or interface (§2.9).

    Otherwise, the Java Virtual Machine must silently ignore the attribute.

    There may be at most one ConstantValue attribute in the attributes table of a field_info structure.
    **/
    ConstantValue {
        /// The value of the constantvalue_index item must be a
        /// valid index into the constant_pool table. The
        /// constant_pool entry at that index gives the
        /// constant value represented by this attribute.
        /// The constant_pool entry must be of a type
        /// appropriate to the field.
        constantvalue_index: u16,
    },
    /**
    The Code attribute is a variable-length attribute in the attributes table of a method_info structure (§4.6). A Code attribute contains the Java Virtual Machine instructions and auxiliary information for a method, including an instance initialization method or a class or interface initialization method (§2.9).

    If the method is either native or abstract, its method_info structure must not have a Code attribute in its attributes table. Otherwise, its method_info structure must have exactly one Code attribute in its attributes table.
    **/
    Code {
        /// The value of the max_stack item gives the maximum
        /// depth of the operand stack of this method
        /// at any point during execution of the method.
        max_stack: u16,
        /// The value of the max_locals item gives the number of
        /// local variables in the local variable array allocated
        /// upon invocation of this method (§2.6.1), including the
        /// local variables used to pass parameters to the method
        /// on its invocation.
        ///
        /// The greatest local variable index for a value of type
        /// long or double is max_locals - 2. The greatest local
        /// variable index for a value of any other type
        /// is max_locals - 1.
        max_locals: u16,
        /**
        The code array gives the actual bytes of Java Virtual Machine code
        that implement the method.

        When the code array is read into memory on a byte-addressable machine,
        if the first byte of the array is aligned on a 4-byte boundary, the
        tableswitch and lookupswitch 32-bit offsets will be 4-byte aligned.
        (Refer to the descriptions of those instructions for more
        information on the consequences of code array alignment.)
        **/
        code: InstructionList,
        /// Each entry in the exception_table array describes one
        /// exception handler in the code array. The order of the
        /// handlers in the exception_table array is significant.
        exception_table: Vec<ExceptionTableEntry>,
        /// Each value of the attributes table must be an attribute_info structure (§4.7).
        /// A Code attribute can have any number of optional attributes associated with it.
        attributes: AttributesCollection,
    },
    /// The StackMapTable attribute is a variable-length attribute in the
    /// attributes table of a Code attribute (§4.7.3).
    ///
    /// A StackMapTable attribute is used during the process of verification by type checking (§4.10.1).
    ///
    /// There may be at most one StackMapTable attribute in the attributes table of a Code attribute.
    ///
    /// In a class file whose version number is 50.0 or above, if a method's Code attribute
    /// does not have a StackMapTable attribute, it has an implicit stack map attribute (§4.10.1).
    /// This implicit stack map attribute is equivalent to a StackMapTable attribute with
    /// number_of_entries equal to zero.
    StackMapTable { entries: Vec<StackMapFrame> },
    /// The Exceptions attribute is a variable-length attribute in the attributes table of a
    /// method_info structure (§4.6). The Exceptions attribute indicates which checked exceptions
    /// a method may throw.
    ///
    /// There may be at most one Exceptions attribute in the attributes table of a method_info structure.
    Exceptions {
        /// Each value in the exception_index_table array must be a valid index
        /// into the constant_pool table. The constant_pool entry at that index
        /// must be a CONSTANT_Class_info structure (§4.4.1) representing a
        /// class type that this method is declared to throw.
        exception_index_table: Vec<u16>,
    },
    /// The BootstrapMethods attribute is a variable-length attribute
    /// in the attributes table of a ClassFile structure (§4.1).
    ///
    /// The BootstrapMethods attribute records bootstrap method
    /// specifiers referenced by invokedynamic instructions (§invokedynamic).
    ///
    /// There must be exactly one BootstrapMethods attribute in
    /// the attributes table of a ClassFile structure if the
    /// constant_pool table of the ClassFile structure has
    /// at least one CONSTANT_InvokeDynamic_info entry (§4.4.10).
    ///
    /// There may be at most one BootstrapMethods attribute in
    /// the attributes table of a ClassFile structure.
    BootstrapMethods {
        /// Each entry in the bootstrap_methods table contains an index
        /// to a CONSTANT_MethodHandle_info structure (§4.4.8)
        /// which specifies a bootstrap method, and a sequence (perhaps empty)
        /// of indexes to static arguments for the bootstrap method.
        bootstrap_methods: Vec<BootstrapMethodsElement>,
    },
    /// The InnerClasses attribute is a variable-length attribute
    /// in the attributes table of a ClassFile structure (§4.1).
    ///
    /// If the constant pool of a class or interface C contains
    /// at least one CONSTANT_Class_info entry (§4.4.1) which
    /// represents a class or interface that is not a member
    /// of a package, then there must be exactly one InnerClasses
    /// attribute in the attributes table of
    /// the ClassFile structure for C.
    InnerClasses {
        /// Every CONSTANT_Class_info entry in the constant_pool table which
        /// represents a class or interface C that is not a package member
        /// must have exactly one corresponding entry in the classes array.
        ///
        /// If a class or interface has members that are classes or interfaces,
        /// its constant_pool table (and hence its InnerClasses attribute)
        /// must refer to each such member (JLS §13.1), even if that member
        /// is not otherwise mentioned by the class.
        ///
        /// In addition, the constant_pool table of every nested class and
        /// nested interface must refer to its enclosing class, so altogether,
        /// every nested class and nested interface will have InnerClasses
        /// information for each enclosing class and for each of its
        /// own nested classes and interfaces.
        classes: Vec<ClassArrayEntry>,
    },
    /// The EnclosingMethod attribute is a fixed-length attribute in the attributes
    /// table of a ClassFile structure (§4.1). A class must have an EnclosingMethod
    /// attribute if and only if it represents a local class or an
    /// anonymous class (JLS §14.3, JLS §15.9.5).
    ///
    /// There may be at most one EnclosingMethod attribute
    /// in the attributes table of a ClassFile structure.
    EnclosingMethod {
        /// The value of the class_index item must be a valid index
        /// into the constant_pool table. The constant_pool entry
        /// at that index must be a CONSTANT_Class_info structure (§4.4.1)
        /// representing the innermost class that encloses the
        /// declaration of the current class.
        class_index: u16,
        /// If the current class is not immediately enclosed by a
        /// method or constructor, then the value of the
        /// method_index item must be zero.
        ///
        /// *In particular, method_index must be zero if the current*
        /// *class was immediately enclosed in source code by an*
        /// *instance initializer, static initializer, instance*
        /// *variable initializer, or class variable initializer.*
        ///
        /// *(The first two concern both local classes and*
        /// *anonymous classes, while the last two concern anonymous*
        /// *classes declared on the right hand side of a*
        /// *field assignment.)*
        ///
        /// Otherwise, the value of the method_index item must be a valid index into the constant_pool table. The constant_pool entry at that index must be a CONSTANT_NameAndType_info structure (§4.4.6) representing the name and type of a method in the class referenced by the class_index attribute above.
        /// It is the responsibility of a Java compiler to ensure that the method identified via the method_index is indeed the closest lexically enclosing method of the class that contains this EnclosingMethod attribute.
        method_index: u16,
    },
    /// The Synthetic attribute is a fixed-length attribute in the
    /// attributes table of a ClassFile, field_info, or method_info
    /// structure (§4.1, §4.5, §4.6). A class member that does not
    /// appear in the source code must be marked using a Synthetic
    /// attribute, or else it must have its ACC_SYNTHETIC flag set.
    /// The only exceptions to this requirement are
    /// compiler-generated methods which are not considered
    /// implementation artifacts, namely the instance initialization
    /// method representing a default constructor of the Java
    /// programming language (§2.9), the class initialization method (§2.9),
    /// and the Enum.values() and Enum.valueOf() methods.
    ///
    /// *The Synthetic attribute was introduced in JDK 1.1 to support nested classes and interfaces.*
    Synthetic,
    /// The Signature attribute is a fixed-length attribute in the attributes table of a ClassFile,
    /// field_info, or method_info structure (§4.1, §4.5, §4.6). A Signature attribute records a
    /// signature (§4.7.9.1) for a class, interface, constructor, method, or field whose
    /// declaration in the Java programming language uses type variables or parameterized types.
    /// See The Java Language Specification, Java SE 8 Edition for details about these types.
    Signature {
        /// The value of the signature_index item must be a valid index
        /// into the constant_pool table. The constant_pool entry at that
        /// index must be a CONSTANT_Utf8_info structure (§4.4.7)
        /// representing a class signature if this Signature attribute
        /// is an attribute of a ClassFile structure; a method signature
        /// if this Signature attribute is an attribute of a
        /// method_info structure; or a field signature otherwise.
        signature_index: u16,
    },
    /// The RuntimeVisibleAnnotations attribute is a variable-length
    /// attribute in the attributes table of a ClassFile, field_info,
    /// or method_info structure (§4.1, §4.5, §4.6).
    ///
    /// The RuntimeVisibleAnnotations attribute records run-time
    /// visible annotations on the declaration of the
    /// corresponding class, field, or method. The
    /// Java Virtual Machine must make these annotations
    /// available so they can be returned by the appropriate reflective APIs.
    ///
    /// There may be at most one RuntimeVisibleAnnotations attribute in the
    /// attributes table of a ClassFile, field_info, or method_info structure.
    RuntimeVisibleAnnotations {
        /// Each entry in the annotations table represents a
        /// single run-time visible annotation on a declaration.
        annotations: Vec<Annotation>,
    },
    /// The RuntimeInvisibleAnnotations attribute is a variable-length
    /// attribute in the attributes table of a ClassFile, field_info,
    /// or method_info structure (§4.1, §4.5, §4.6).
    ///
    /// The RuntimeInvisibleAnnotations attribute records run-time
    /// invisible annotations on the declaration of the
    /// corresponding class, method, or field.
    ///
    /// There may be at most one RuntimeInvisibleAnnotations attribute
    /// in the attributes table of a ClassFile, field_info,
    /// or method_info structure.
    ///
    /// The RuntimeInvisibleAnnotations attribute is similar to the
    /// RuntimeVisibleAnnotations attribute (§4.7.16), except that
    /// the annotations represented by a RuntimeInvisibleAnnotations
    /// attribute must not be made available for return by reflective APIs,
    /// unless the Java Virtual Machine has been instructed to retain these
    /// annotations via some implementation-specific mechanism such as a command
    /// line flag. In the absence of such instructions,
    /// the Java Virtual Machine ignores this attribute.
    RuntimeInvisibleAnnotations {
        /// Each entry in the annotations table represents a
        /// single run-time invisible annotation on a declaration.
        annotations: Vec<Annotation>,
    },
    /// The RuntimeVisibleParameterAnnotations attribute is a
    /// variable-length attribute in the attributes table of
    /// the method_info structure (§4.6).
    ///
    /// The RuntimeVisibleParameterAnnotations attribute records
    /// run-time visible annotations on the declarations of formal
    /// parameters of the corresponding method. The
    /// Java Virtual Machine must make these annotations
    /// available so they can be returned by
    /// the appropriate reflective APIs.
    ///
    /// There may be at most one RuntimeVisibleParameterAnnotations
    /// attribute in the attributes table of a method_info structure.
    RuntimeVisibleParameterAnnotations {
        /// Each entry in the parameter_annotations table represents
        /// all of the run-time visible annotations on the declaration
        /// of a single formal parameter. The i'th entry in the table
        /// corresponds to the i'th formal parameter in
        /// the method descriptor.
        parameter_annotations: Vec<ParameterAnnotation>,
    },
    /// The RuntimeInvisibleParameterAnnotations attribute is a
    /// variable-length attribute in the attributes table of a
    /// method_info structure (§4.6).
    ///
    /// The RuntimeInvisibleParameterAnnotations attribute records
    /// run-time invisible annotations on the declarations of formal
    /// parameters of the corresponding method.
    ///
    /// There may be at most one RuntimeInvisibleParameterAnnotations
    /// attribute in the attributes table of a method_info structure.
    ///
    /// The RuntimeInvisibleParameterAnnotations attribute is similar
    /// to the RuntimeVisibleParameterAnnotations attribute (§4.7.18),
    /// except that the annotations represented by a
    /// RuntimeInvisibleParameterAnnotations attribute must not be made
    /// available for return by reflective APIs,
    /// unless the Java Virtual Machine has specifically been instructed
    /// to retain these annotations via some implementation-specific mechanism
    /// such as a command line flag.
    ///
    /// In the absence of such instructions,
    /// the Java Virtual Machine ignores this attribute.
    RuntimeInvisibleParameterAnnotations {
        /// Each entry in the parameter_annotations table represents
        /// all of the run-time invisible annotations on the
        /// declaration of a single formal parameter.
        ///
        /// The i'th entry in the table corresponds to
        /// the i'th formal parameter in
        /// the method descriptor.
        parameter_annotations: Vec<ParameterAnnotation>,
    },
    /// The RuntimeVisibleTypeAnnotations attribute is an variable-length
    /// attribute in the attributes table of a ClassFile, field_info,
    /// or method_info structure, or Code attribute (§4.1, §4.5, §4.6, §4.7.3).
    ///
    /// The RuntimeVisibleTypeAnnotations attribute records run-time visible annotations
    /// on types used in the declaration of the corresponding class, field, or method,
    /// or in an expression in the corresponding method body.
    ///
    /// The RuntimeVisibleTypeAnnotations attribute also records run-time visible annotations
    /// on type parameter declarations of generic classes, interfaces, methods, and constructors.
    ///
    /// The Java Virtual Machine must make these annotations available so they can
    /// be returned by the appropriate reflective APIs.
    ///
    /// There may be at most one RuntimeVisibleTypeAnnotations attribute in the attributes table of a
    /// ClassFile, field_info, or method_info structure, or Code attribute.
    ///
    /// An attributes table contains a RuntimeVisibleTypeAnnotations attribute only if types are annotated
    /// in kinds of declaration or expression that correspond to the parent structure or attribute of the attributes table.
    RuntimeVisibleTypeAnnotations {
        /// Each entry in the annotations table represents a single run-time
        /// visible annotation on a type used in a declaration or expression.
        annotations: Vec<TypeAnnotation>,
    },
    /// The RuntimeInvisibleTypeAnnotations attribute is an variable-length attribute
    /// in the attributes table of a ClassFile, field_info, or method_info structure,
    /// or Code attribute (§4.1, §4.5, §4.6, §4.7.3).
    ///
    /// The RuntimeInvisibleTypeAnnotations attribute records run-time invisible
    /// annotations on types used in the corresponding declaration of a class,
    /// field, or method, or in an expression in the corresponding method body.
    ///
    /// The RuntimeInvisibleTypeAnnotations attribute also records annotations
    /// on type parameter declarations of generic classes,
    /// interfaces, methods, and constructors.
    ///
    /// There may be at most one RuntimeInvisibleTypeAnnotations attribute in the
    /// attributes table of a ClassFile, field_info, or method_info
    /// structure, or Code attribute.
    ///
    /// An attributes table contains a RuntimeInvisibleTypeAnnotations attribute only if
    /// types are annotated in kinds of declaration or expression that correspond
    /// to the parent structure or attribute of the attributes table.
    RuntimeInvisibleTypeAnnotations {
        /// Each entry in the annotations table represents a
        /// single run-time invisible annotation on a
        /// type used in a declaration or expression.
        annotations: Vec<TypeAnnotation>,
    },
    /// The AnnotationDefault attribute is a variable-length
    /// attribute in the attributes table of certain method_info
    /// structures (§4.6), namely those representing elements of
    /// annotation types (JLS §9.6.1). The AnnotationDefault
    /// attribute records the default value (JLS §9.6.2) for
    /// the element represented by the method_info structure.
    ///
    /// The Java Virtual Machine must make this default value
    /// available so it can be applied by appropriate reflective APIs.
    ///
    /// There may be at most one AnnotationDefault attribute in the
    /// attributes table of a method_info structure which
    /// represents an element of an annotation type.
    AnnotationDefault {
        /// The default_value item represents the default
        /// value of the annotation type element represented
        /// by the method_info structure enclosing
        /// this AnnotationDefault attribute.
        default_value: ElementValue,
    },
    /// The MethodParameters attribute is a variable-length
    /// attribute in the attributes table of a method_info
    /// structure (§4.6).
    ///
    /// A MethodParameters attribute records information
    /// about the formal parameters of a method,
    /// such as their names.
    ///
    /// There may be at most one MethodParameters attribute
    /// in the attributes table of a method_info structure.
    MethodParameters {
        parameters: Vec<MethodParametersElement>,
    },
    /// The SourceFile attribute is an optional fixed-length
    /// attribute in the attributes table of a ClassFile
    /// structure (§4.1).
    ///
    /// There may be at most one SourceFile attribute in the
    /// attributes table of a ClassFile structure.
    SourceFile {
        /// The value of the sourcefile_index item must be a
        /// valid index into the constant_pool table. The
        /// constant_pool entry at that index must be a
        /// CONSTANT_Utf8_info structure (§4.4.7)
        /// representing a string.
        ///
        /// *The string referenced by the sourcefile_index item will be*
        /// *interpreted as indicating the name of the source file from*
        /// *which this class file was compiled. It will not be*
        /// *interpreted as indicating the name of a directory containing*
        /// *the file or an absolute path name for the file;*
        /// *such platform-specific additional information must be supplied*
        /// *by the run-time interpreter or development tool at the time*
        /// *the file name is actually used.*
        sourcefile_index: u16,
    },
    /// The SourceDebugExtension attribute is an optional
    /// attribute in the attributes table of
    /// a ClassFile structure (§4.1).
    ///
    /// There may be at most one SourceDebugExtension
    /// attribute in the attributes table
    /// of a ClassFile structure.
    SourceDebugExtension {
        /// The debug_extension array holds extended debugging information
        /// which has no semantic effect on the Java Virtual Machine.
        /// The information is represented using a modified UTF-8 string
        /// (§4.4.7) with no terminating zero byte.
        debug_extension: Vec<u8>,
    },
    /// The LineNumberTable attribute is an optional variable-length
    /// attribute in the attributes table of a Code attribute (§4.7.3).
    /// It may be used by debuggers to determine which part of the code
    /// array corresponds to a given line number in the original source file.
    ///
    /// If multiple LineNumberTable attributes are present in the attributes
    /// table of a Code attribute, then they may appear in any order.
    /// There may be more than one LineNumberTable attribute per line of a
    /// source file in the attributes table of a Code attribute. That is,
    /// LineNumberTable attributes may together represent a given line
    /// of a source file, and need not be one-to-one with source lines.
    LineNumberTable {
        /// Each entry in the line_number_table array indicates
        /// that the line number in the original source file
        /// changes at a given point in the code array.
        line_number_table: Vec<LineNumberTableEntry>,
    },
    /// The LocalVariableTable attribute is an optional
    /// variable-length attribute in the attributes
    /// table of a Code attribute (§4.7.3).
    ///
    /// It may be used by debuggers to determine the
    /// value of a given local variable during
    /// the execution of a method.
    ///
    /// If multiple LocalVariableTable attributes are present
    /// in the attributes table of a Code attribute,
    /// then they may appear in any order.
    ///
    /// There may be no more than one LocalVariableTable attribute
    /// per local variable in the attributes table of a Code attribute.
    LocalVariableTable {
        /// Each entry in the local_variable_table array indicates
        /// a range of code array offsets within which a local
        /// variable has a value. It also indicates the index
        /// into the local variable array of the current frame
        /// at which that local variable can be found.
        local_variable_table: Vec<LocalVariableTableEntry>,
    },
    /// The LocalVariableTypeTable attribute is an optional
    /// variable-length attribute in the attributes table
    /// of a Code attribute (§4.7.3).
    ///
    /// It may be used by debuggers to determine the value of a
    /// given local variable during the execution of a method.
    ///
    /// If multiple LocalVariableTypeTable attributes are present
    /// in the attributes table of a given Code attribute,
    /// then they may appear in any order.
    ///
    /// There may be no more than one LocalVariableTypeTable attribute
    /// per local variable in the attributes table of a Code attribute.
    ///
    /// The LocalVariableTypeTable attribute differs from the LocalVariableTable
    /// attribute (§4.7.13) in that it provides signature information rather
    /// than descriptor information. This difference is only significant for
    /// variables whose type uses a type variable or parameterized type.
    ///
    /// Such variables will appear in both tables, while variables of
    /// other types will appear only in LocalVariableTable.
    LocalVariableTypeTable {
        /// Each entry in the local_variable_type_table array
        /// indicates a range of code array offsets within
        /// which a local variable has a value.
        ///
        /// It also indicates the index into the local
        /// variable array of the current frame at which
        /// that local variable can be found.
        local_variable_type_table: Vec<LocalVariableTypeTableEntry>,
    },
    /// The Deprecated attribute is an optional fixed-length
    /// attribute in the attributes table of a ClassFile,
    /// field_info, or method_info structure (§4.1, §4.5, §4.6).
    ///
    /// A class, interface, method, or field may be marked using a
    /// Deprecated attribute to indicate that the class, interface,
    /// method, or field has been superseded.
    ///
    /// A run-time interpreter or tool that reads the class
    /// file format, such as a compiler, can use this marking
    /// to advise the user that a superseded class, interface,
    /// method, or field is being referred to. The presence of
    /// a Deprecated attribute does not alter the
    /// semantics of a class or interface.
    Deprecated,
}

/// Collection of all attributes.
#[derive(Debug)]
pub struct AttributesCollection {
    pub collection: HashMap<String, Vec<Attributes>>
}
impl AttributesCollection {
    /// Insert an attribute in to the collection.
    fn insert(&mut self, k: String, v: Attributes) {
        self.collection.entry(k).or_default().push(v);
    }

    pub fn get(&self, k: &str) -> &[Attributes] {
        self.collection.get(k).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn take(&mut self, k: &str) -> Vec<Attributes> {
        self.collection.remove(k).unwrap_or_default()
    }
}

impl ClassFileItem for AttributesCollection {
    fn read_from_stream<R: Read>(s: &mut ClassFileStream<R>, cp: Option<&ConstantPool>) -> error::Result<Self>
    where
        Self: Sized {
        let attributes_count = s.read_u2()?;
        let mut attributes = Self {
            collection: HashMap::new()
        };
        for _ in 0..attributes_count {
            let cp = cp.expect("constant pool should exist at the time of attribute deserialization");
            let attribute_name_index = s.read_u2()?;
            let attribute_length = s.read_u4()?;
            let mut info = Cursor::new(s.read_dynamic(attribute_length as usize)?);
    
            let mut s = ClassFileStream::new(&mut info);
    
            let attribute_name = cp.get_utf8_constant(attribute_name_index as usize)?;
    
            let a = match attribute_name {
                ConstantValue => Ok(Attributes::ConstantValue {
                    constantvalue_index: s.read_u2()?,
                }),
                Code => {
                    let max_stack = s.read_u2()?;
                    let max_locals = s.read_u2()?;
                    let code_length = s.read_u4()?;
                    let code = s.read_sequence::<u8>(Some(cp), code_length as usize)?;
                    let exception_table_length = s.read_u2()?;
                    let exception_table = s.read_sequence::<ExceptionTableEntry>(
                        Some(cp),
                        exception_table_length as usize,
                    )?;
                    let attributes = AttributesCollection::read_from_stream(&mut s, Some(cp))?;
    
                    let code = InstructionList::read_from_stream(
                        &mut ClassFileStream::new(&mut Cursor::new(code)),
                        Some(cp),
                    )?;
                    Ok(Attributes::Code {
                        max_stack,
                        max_locals,
                        code,
                        exception_table,
                        attributes,
                    })
                }
                StackMapTable => {
                    let number_of_entries = s.read_u2()?;
                    let entries = s.read_sequence(Some(cp), number_of_entries as usize)?;
                    Ok(Attributes::StackMapTable { entries })
                }
                Exceptions => {
                    let number_of_exceptions = s.read_u2()?;
                    let exception_index_table =
                        s.read_sequence(Some(cp), number_of_exceptions as usize)?;
                    Ok(Attributes::Exceptions {
                        exception_index_table,
                    })
                }
                InnerClasses => {
                    let number_of_classes = s.read_u2()?;
                    Ok(Attributes::InnerClasses {
                        classes: s.read_sequence(Some(cp), number_of_classes as usize)?,
                    })
                }
                EnclosingMethod => Ok(Attributes::EnclosingMethod {
                    class_index: s.read_u2()?,
                    method_index: s.read_u2()?,
                }),
                Synthetic => Ok(Attributes::Synthetic),
                Signature => Ok(Attributes::Signature {
                    signature_index: s.read_u2()?,
                }),
                SourceFile => Ok(Attributes::SourceFile {
                    sourcefile_index: s.read_u2()?,
                }),
                SourceDebugExtension => {
                    let bytes = s.read_dynamic(attribute_length as usize)?;
                    Ok(Attributes::SourceDebugExtension {
                        debug_extension: bytes,
                    })
                }
                LineNumberTable => {
                    let line_number_table_length = s.read_u2()?;
                    Ok(Attributes::LineNumberTable {
                        line_number_table: s
                            .read_sequence(Some(cp), line_number_table_length as usize)?,
                    })
                }
                LocalVariableTable => {
                    let local_variable_table_length = s.read_u2()?;
                    Ok(Attributes::LocalVariableTable {
                        local_variable_table: s
                            .read_sequence(Some(cp), local_variable_table_length as usize)?,
                    })
                }
                LocalVariableTypeTable => {
                    let local_variable_type_table_length = s.read_u2()?;
                    Ok(Attributes::LocalVariableTypeTable {
                        local_variable_type_table: s
                            .read_sequence(Some(cp), local_variable_type_table_length as usize)?,
                    })
                }
                Deprecated => Ok(Attributes::Deprecated),
                RuntimeVisibleAnnotations => {
                    let num_annotations = s.read_u2()?;
                    Ok(Attributes::RuntimeVisibleAnnotations {
                        annotations: s.read_sequence(Some(cp), num_annotations as usize)?,
                    })
                }
                RuntimeInvisibleAnnotations => {
                    let num_annotations = s.read_u2()?;
                    Ok(Attributes::RuntimeInvisibleAnnotations {
                        annotations: s.read_sequence(Some(cp), num_annotations as usize)?,
                    })
                }
                RuntimeVisibleParameterAnnotations => {
                    let num_parameters = s.read_u1()?;
                    Ok(Attributes::RuntimeVisibleParameterAnnotations {
                        parameter_annotations: s.read_sequence(Some(cp), num_parameters as usize)?,
                    })
                }
                RuntimeInvisibleParameterAnnotations => {
                    let num_parameters = s.read_u1()?;
                    Ok(Attributes::RuntimeInvisibleParameterAnnotations {
                        parameter_annotations: s.read_sequence(Some(cp), num_parameters as usize)?,
                    })
                }
                RuntimeVisibleTypeAnnotations => {
                    let num_annotations = s.read_u2()?;
                    Ok(Attributes::RuntimeVisibleTypeAnnotations {
                        annotations: s.read_sequence(Some(cp), num_annotations as usize)?,
                    })
                }
                RuntimeInvisibleTypeAnnotations => {
                    let num_annotations = s.read_u2()?;
                    Ok(Attributes::RuntimeInvisibleTypeAnnotations {
                        annotations: s.read_sequence(Some(cp), num_annotations as usize)?,
                    })
                }
                AnnotationDefault => Ok(Attributes::AnnotationDefault {
                    default_value: ElementValue::read_from_stream(&mut s, Some(cp))?,
                }),
                BootstrapMethods => {
                    let num_bootstrap_methods = s.read_u2()?;
                    Ok(Attributes::BootstrapMethods {
                        bootstrap_methods: s.read_sequence(Some(cp), num_bootstrap_methods as usize)?,
                    })
                }
                MethodParameters => {
                    let parameters_count = s.read_u1()?;
                    Ok(Attributes::MethodParameters {
                        parameters: s.read_sequence(Some(cp), parameters_count as usize)?,
                    })
                }
                v => Err(ClassFileError::UnknownAttribute(v.to_string())),
            }?;
            attributes.insert(attribute_name.to_string(), a);
        };
        Ok(attributes)
    }
}

// impl ClassFileItem for Attributes {
//     fn read_from_stream<R: Read>(
//         s: &mut ClassFileStream<R>,
//         cp: Option<&ConstantPool>,
//     ) -> error::Result<Self>
//     where
//         Self: std::marker::Sized,
//     {
//         let cp = cp.expect("constant pool should exist at the time of attribute deserialization");
//         let attribute_name_index = s.read_u2()?;
//         let attribute_length = s.read_u4()?;
//         let mut info = Cursor::new(s.read_dynamic(attribute_length as usize)?);

//         let mut s = ClassFileStream::new(&mut info);

//         let attribute_name = cp.get_utf8_constant(attribute_name_index as usize)?;

//         match attribute_name {
//             ConstantValue => Ok(Self::ConstantValue {
//                 constantvalue_index: s.read_u2()?,
//             }),
//             Code => {
//                 let max_stack = s.read_u2()?;
//                 let max_locals = s.read_u2()?;
//                 let code_length = s.read_u4()?;
//                 let code = s.read_sequence::<u8>(Some(cp), code_length as usize)?;
//                 let exception_table_length = s.read_u2()?;
//                 let exception_table = s.read_sequence::<ExceptionTableEntry>(
//                     Some(cp),
//                     exception_table_length as usize,
//                 )?;
//                 let attributes_count = s.read_u2()?;
//                 let attributes =
//                     s.read_sequence::<Attributes>(Some(cp), attributes_count as usize)?;

//                 let code = OpcodeList::read_from_stream(
//                     &mut ClassFileStream::new(&mut Cursor::new(code)),
//                     Some(cp),
//                 )?;
//                 Ok(Self::Code {
//                     max_stack,
//                     max_locals,
//                     code,
//                     exception_table,
//                     attributes,
//                 })
//             }
//             StackMapTable => {
//                 let number_of_entries = s.read_u2()?;
//                 let entries = s.read_sequence(Some(cp), number_of_entries as usize)?;
//                 Ok(Self::StackMapTable { entries })
//             }
//             Exceptions => {
//                 let number_of_exceptions = s.read_u2()?;
//                 let exception_index_table =
//                     s.read_sequence(Some(cp), number_of_exceptions as usize)?;
//                 Ok(Self::Exceptions {
//                     exception_index_table,
//                 })
//             }
//             InnerClasses => {
//                 let number_of_classes = s.read_u2()?;
//                 Ok(Self::InnerClasses {
//                     classes: s.read_sequence(Some(cp), number_of_classes as usize)?,
//                 })
//             }
//             EnclosingMethod => Ok(Self::EnclosingMethod {
//                 class_index: s.read_u2()?,
//                 method_index: s.read_u2()?,
//             }),
//             Synthetic => Ok(Self::Synthetic),
//             Signature => Ok(Self::Signature {
//                 signature_index: s.read_u2()?,
//             }),
//             SourceFile => Ok(Self::SourceFile {
//                 sourcefile_index: s.read_u2()?,
//             }),
//             SourceDebugExtension => {
//                 let bytes = s.read_dynamic(attribute_length as usize)?;
//                 Ok(Self::SourceDebugExtension {
//                     debug_extension: bytes,
//                 })
//             }
//             LineNumberTable => {
//                 let line_number_table_length = s.read_u2()?;
//                 Ok(Self::LineNumberTable {
//                     line_number_table: s
//                         .read_sequence(Some(cp), line_number_table_length as usize)?,
//                 })
//             }
//             LocalVariableTable => {
//                 let local_variable_table_length = s.read_u2()?;
//                 Ok(Self::LocalVariableTable {
//                     local_variable_table: s
//                         .read_sequence(Some(cp), local_variable_table_length as usize)?,
//                 })
//             }
//             LocalVariableTypeTable => {
//                 let local_variable_type_table_length = s.read_u2()?;
//                 Ok(Self::LocalVariableTypeTable {
//                     local_variable_type_table: s
//                         .read_sequence(Some(cp), local_variable_type_table_length as usize)?,
//                 })
//             }
//             Deprecated => Ok(Self::Deprecated),
//             RuntimeVisibleAnnotations => {
//                 let num_annotations = s.read_u2()?;
//                 Ok(Self::RuntimeVisibleAnnotations {
//                     annotations: s.read_sequence(Some(cp), num_annotations as usize)?,
//                 })
//             }
//             RuntimeInvisibleAnnotations => {
//                 let num_annotations = s.read_u2()?;
//                 Ok(Self::RuntimeInvisibleAnnotations {
//                     annotations: s.read_sequence(Some(cp), num_annotations as usize)?,
//                 })
//             }
//             RuntimeVisibleParameterAnnotations => {
//                 let num_parameters = s.read_u1()?;
//                 Ok(Self::RuntimeVisibleParameterAnnotations {
//                     parameter_annotations: s.read_sequence(Some(cp), num_parameters as usize)?,
//                 })
//             }
//             RuntimeInvisibleParameterAnnotations => {
//                 let num_parameters = s.read_u1()?;
//                 Ok(Self::RuntimeInvisibleParameterAnnotations {
//                     parameter_annotations: s.read_sequence(Some(cp), num_parameters as usize)?,
//                 })
//             }
//             RuntimeVisibleTypeAnnotations => {
//                 let num_annotations = s.read_u2()?;
//                 Ok(Self::RuntimeVisibleTypeAnnotations {
//                     annotations: s.read_sequence(Some(cp), num_annotations as usize)?,
//                 })
//             }
//             RuntimeInvisibleTypeAnnotations => {
//                 let num_annotations = s.read_u2()?;
//                 Ok(Self::RuntimeInvisibleTypeAnnotations {
//                     annotations: s.read_sequence(Some(cp), num_annotations as usize)?,
//                 })
//             }
//             AnnotationDefault => Ok(Self::AnnotationDefault {
//                 default_value: ElementValue::read_from_stream(&mut s, Some(cp))?,
//             }),
//             BootstrapMethods => {
//                 let num_bootstrap_methods = s.read_u2()?;
//                 Ok(Self::BootstrapMethods {
//                     bootstrap_methods: s.read_sequence(Some(cp), num_bootstrap_methods as usize)?,
//                 })
//             }
//             MethodParameters => {
//                 let parameters_count = s.read_u1()?;
//                 Ok(Self::MethodParameters {
//                     parameters: s.read_sequence(Some(cp), parameters_count as usize)?,
//                 })
//             }
//             v => Err(ClassFileError::UnknownAttribute(v.to_string())),
//         }
//     }
// }

/// Method parameters element.
#[derive(Debug)]
pub struct MethodParametersElement {
    /// The value of the name_index item must either
    /// be zero or a valid index into the constant_pool table.
    ///
    /// If the value of the name_index item is zero, then
    /// this parameters element indicates a formal
    /// parameter with no name.
    ///
    /// If the value of the name_index item is nonzero,
    /// the constant_pool entry at that index must be
    /// a CONSTANT_Utf8_info structure representing
    /// a valid unqualified name denoting
    /// a formal parameter (§4.2.2).
    pub name_index: u16,
    pub access_flags: FormalParameterAccessFlags,
}

impl ClassFileItem for MethodParametersElement {
    fn read_from_stream<R: Read>(
        s: &mut ClassFileStream<R>,
        cp: Option<&ConstantPool>,
    ) -> error::Result<Self>
    where
        Self: std::marker::Sized,
    {
        let name_index = s.read_u2()?;
        let access_flags = s.read_u2()?;
        Ok(Self {
            name_index,
            access_flags: FormalParameterAccessFlags::from_bits(access_flags)
                .ok_or(ClassFileError::BadFormalParameterAccessFlags)?,
        })
    }
}

bitflags::bitflags! {
    pub struct FormalParameterAccessFlags: u16 {
        /// Indicates that the formal parameter was declared final.
        const ACC_FINAL = 0x0010;
        /// Indicates that the formal parameter was not
        /// explicitly or implicitly declared in source code,
        /// according to the specification of the language in
        /// which the source code was written (JLS §13.1).
        ///
        /// (The formal parameter is an implementation
        /// artifact of the compiler which produced this class file.)
        const ACC_SYNTHETIC = 0x1000;
        /// Indicates that the formal parameter was implicitly
        /// declared in source code, according to the specification
        /// of the language in which the source code was written (JLS §13.1).
        ///
        /// (The formal parameter is mandated by a language
        /// specification, so all compilers for the language must emit it.)
        const ACC_MANDATED = 0x8000;
    }
}

/// Bootstrap methods element.
///
/// Each entry in the bootstrap_methods table
/// contains an index to a CONSTANT_MethodHandle_info
/// structure (§4.4.8) which specifies a bootstrap method,
/// and a sequence (perhaps empty) of indexes to static
/// arguments for the bootstrap method.
#[derive(Debug)]
pub struct BootstrapMethodsElement {
    /// The value of the bootstrap_method_ref item must be
    /// a valid index into the constant_pool table.
    ///
    /// The constant_pool entry at that index must
    /// be a CONSTANT_MethodHandle_info structure (§4.4.8).
    pub bootstrap_method_ref: u16,
    /// Each entry in the bootstrap_arguments array must
    /// be a valid index into the constant_pool table.
    /// The constant_pool entry at that index must be
    /// a CONSTANT_String_info, CONSTANT_Class_info,
    /// CONSTANT_Integer_info, CONSTANT_Long_info,
    /// CONSTANT_Float_info, CONSTANT_Double_info,
    /// CONSTANT_MethodHandle_info,
    /// or CONSTANT_MethodType_info structure.
    pub bootstrap_arguments: Vec<u16>,
}

impl ClassFileItem for BootstrapMethodsElement {
    fn read_from_stream<R: Read>(
        s: &mut ClassFileStream<R>,
        cp: Option<&ConstantPool>,
    ) -> error::Result<Self>
    where
        Self: std::marker::Sized,
    {
        let bootstrap_method_ref = s.read_u2()?;
        let num_bootstrap_arguments = s.read_u2()?;
        Ok(Self {
            bootstrap_method_ref,
            bootstrap_arguments: s.read_sequence(cp, num_bootstrap_arguments as usize)?,
        })
    }
}

#[derive(Debug)]
/// Type annotation.
///
/// Each type annotation structure represents a
/// single run-time visible annotation on a type
/// used in a declaration or expression.
pub struct TypeAnnotation {
    /// The value of the target_info item denotes
    /// precisely which type in a declaration
    /// or expression is annotated.
    pub target_info: TargetInfoType,
    /// The value of the target_path item denotes
    /// precisely which part of the type
    /// indicated by target_info is annotated.
    pub target_path: TypePath,
    /// The meaning of this item in the type_annotation
    /// structure is the same as its meaning
    /// in the annotation structure.
    pub type_index: u16,
    /// The meaning of this item in the type_annotation
    /// structure is the same as its meaning
    /// in the annotation structure.
    pub element_value_pairs: Vec<ElementValuePairElement>,
}

impl ClassFileItem for TypeAnnotation {
    fn read_from_stream<R: Read>(
        s: &mut ClassFileStream<R>,
        cp: Option<&ConstantPool>,
    ) -> error::Result<Self>
    where
        Self: std::marker::Sized,
    {
        let target_info = TargetInfoType::read_from_stream(s, cp)?;
        let target_path = TypePath::read_from_stream(s, cp)?;
        let type_index = s.read_u2()?;
        let num_element_value_pairs = s.read_u2()?;
        Ok(Self {
            target_info,
            target_path,
            type_index,
            element_value_pairs: s.read_sequence(cp, num_element_value_pairs as usize)?,
        })
    }
}

#[derive(Debug)]
/// Type path.
/// If the value of path_length is 0, then the
/// annotation appears directly on the type itself.
///
/// If the value of path_length is non-zero, then
/// each entry in the path array represents an
/// iterative, left-to-right step towards the
/// precise location of the annotation in an
/// array type, nested type, or parameterized
/// type. (In an array type, the iteration visits
/// the array type itself, then its component type,
/// then the component type of that component type,
/// and so on, until the element type is reached.)
pub struct TypePath {
    pub path: Vec<TypePathEntry>,
}

impl ClassFileItem for TypePath {
    fn read_from_stream<R: Read>(
        s: &mut ClassFileStream<R>,
        cp: Option<&ConstantPool>,
    ) -> error::Result<Self>
    where
        Self: std::marker::Sized,
    {
        let path_length = s.read_u2()?;
        Ok(Self {
            path: s.read_sequence(cp, path_length as usize)?,
        })
    }
}

#[derive(Debug)]
/// Type path entry.
pub struct TypePathEntry {
    pub type_path_kind: TypePathKind,
    /// If the value of the type_path_kind item is AnnotationDeeperArray, AnnotationDeeperNested, or AnnotationBoundWildcardParameterizedType,
    /// then the value of the type_argument_index item is 0.
    ///
    /// If the value of the type_path_kind item is AnnotationTypeArgParameterizedType, then
    /// the value of the type_argument_index item specifies
    /// which type argument of a parameterized type is annotated,
    /// where 0 indicates the first type argument of a parameterized type.
    pub type_argument_index: u8,
}

impl ClassFileItem for TypePathEntry {
    fn read_from_stream<R: Read>(
        s: &mut ClassFileStream<R>,
        cp: Option<&ConstantPool>,
    ) -> error::Result<Self>
    where
        Self: std::marker::Sized,
    {
        Ok(Self {
            type_path_kind: TypePathKind::from_u8(s.read_u1()?)?,
            type_argument_index: s.read_u1()?,
        })
    }
}

/// Type path kinds.
mod typepathkinds {
    use crate::error::{self, ClassFileError};

    pub const ANNOTATION_DEEPER_ARRAY: u8 = 0;
    pub const ANNOTATION_DEEPER_NESTED: u8 = 1;
    pub const ANNOTATION_BOUND_WILDCARD_PARAMETERIZED_TYPE: u8 = 2;
    pub const ANNOTATION_TYPEARG_PARAMETERIZED_TYPE: u8 = 3;

    #[derive(Debug)]
    /// Type path kind.
    pub enum TypePathKind {
        /// Annotation is deeper in an array type
        AnnotationDeeperArray,
        /// Annotation is deeper in a nested type
        AnnotationDeeperNested,
        /// Annotation is on the bound of a wildcard type argument of a parameterized type
        AnnotationBoundWildcardParameterizedType,
        /// Annotation is on a type argument of a parameterized type
        AnnotationTypeArgParameterizedType,
    }

    impl TypePathKind {
        /// Construct a `TypePathKind` from a u8.
        pub fn from_u8(v: u8) -> error::Result<Self> {
            match v {
                ANNOTATION_DEEPER_ARRAY => Ok(Self::AnnotationDeeperArray),
                ANNOTATION_DEEPER_NESTED => Ok(Self::AnnotationDeeperNested),
                ANNOTATION_BOUND_WILDCARD_PARAMETERIZED_TYPE => {
                    Ok(Self::AnnotationBoundWildcardParameterizedType)
                }
                ANNOTATION_TYPEARG_PARAMETERIZED_TYPE => {
                    Ok(Self::AnnotationTypeArgParameterizedType)
                }
                v => Err(ClassFileError::UnknownTypePathKind(v)),
            }
        }
    }
}

#[derive(Debug)]
/// Target info type.
pub enum TargetInfoType {
    /// The type_parameter_target item indicates that an
    /// annotation appears on the declaration of the i'th
    /// type parameter of a generic class, generic interface,
    /// generic method, or generic constructor.
    TypeParameterTarget {
        /// The value of the type_parameter_index item specifies
        /// which type parameter declaration is annotated.
        ///
        /// A type_parameter_index value of 0 specifies
        /// the first type parameter declaration.
        type_parameter_index: u8,
    },
    /// The supertype_target item indicates that an annotation
    /// appears on a type in the extends or implements clause
    /// of a class or interface declaration.
    SupertypeTarget {
        /// A supertype_index value of 65535 specifies
        /// that the annotation appears on the superclass
        /// in an extends clause of a class declaration.
        ///
        /// Any other supertype_index value is an index
        /// into the interfaces array of the enclosing
        /// ClassFile structure, and specifies that the
        /// annotation appears on that superinterface in
        /// either the implements clause of a class
        /// declaration or the extends clause of an
        /// interface declaration.
        supertype_index: u16,
    },
    /// The type_parameter_bound_target item indicates that
    /// an annotation appears on the i'th bound of the j'th
    /// type parameter declaration of a generic class,
    /// interface, method, or constructor.
    TypeParameterBoundTarget {
        /// The value of the of type_parameter_index item
        /// specifies which type parameter declaration has
        /// an annotated bound. A type_parameter_index value
        /// of 0 specifies the first type parameter declaration.
        type_parameter_index: u8,
        /// The value of the bound_index item specifies which
        /// bound of the type parameter declaration indicated
        /// by type_parameter_index is annotated. A bound_index
        /// value of 0 specifies the first bound
        /// of a type parameter declaration.
        bound_index: u8,
    },
    /// The empty_target item indicates that an
    /// annotation appears on either the type in
    /// a field declaration, the return type of
    /// a method, the type of a newly constructed
    /// object, or the receiver type of
    /// a method or constructor.
    EmptyTarget,
    /// The formal_parameter_target item indicates
    /// that an annotation appears on the type in
    /// a formal parameter declaration of a method,
    /// constructor, or lambda expression.
    FormalParameterTarget {
        /// The value of the formal_parameter_index item
        /// specifies which formal parameter declaration
        /// has an annotated type.
        ///
        /// A formal_parameter_index value of 0 specifies
        /// the first formal parameter declaration.
        formal_parameter_index: u8,
    },
    /// The throws_target item indicates that an annotation
    /// appears on the i'th type in the throws clause
    /// of a method or constructor declaration.
    ThrowsTarget {
        ///  The value of the throws_type_index item is
        /// an index into the exception_index_table array
        /// of the Exceptions attribute of the method_info
        /// structure enclosing the
        /// RuntimeVisibleTypeAnnotations attribute.
        throws_type_index: u8,
    },
    ///  The localvar_target item indicates that an annotation
    /// appears on the type in a local variable declaration,
    /// including a variable declared as
    /// a resource in a try-with-resources statement.
    LocalVarTarget {
        table: Vec<LocalVarTargetTableEntry>,
    },
    /// The catch_target item indicates that an
    /// annotation appears on the i'th type
    /// in an exception parameter declaration.
    CatchTarget {
        /// The value of the exception_table_index item
        /// is an index into the exception_table array
        /// of the Code attribute enclosing the
        /// RuntimeVisibleTypeAnnotations attribute.
        exception_table_index: u16,
    },
    /// The offset_target item indicates that an annotation appears
    /// on either the type in an instanceof expression or a new expression,
    /// or the type before the :: in a method reference expression.
    OffsetTarget {
        /// The value of the offset item specifies the
        /// code array offset of either the instanceof
        /// bytecode instruction corresponding to the
        /// instanceof expression, the new bytecode
        /// instruction corresponding to the new
        /// expression, or the bytecode instruction
        /// corresponding to the method reference expression.
        offset: u16,
    },
    /// The type_argument_target item indicates that an
    /// annotation appears either on the i'th type in a
    /// cast expression, or on the i'th type argument
    /// in the explicit type argument list for any of
    /// the following: a new expression, an explicit
    /// constructor invocation statement, a method
    /// invocation expression, or a method
    /// reference expression.
    TypeArgumentTarget {
        /// The value of the offset item specifies the code
        /// array offset of either the bytecode instruction
        /// corresponding to the cast expression,
        /// the new bytecode instruction corresponding to
        /// the new expression, the bytecode instruction
        /// corresponding to the explicit constructor invocation
        /// statement, the bytecode instruction corresponding to
        /// the method invocation expression, or the bytecode
        /// instruction corresponding to the method reference expression.
        offset: u16,
        /// For a cast expression, the value of the type_argument_index item
        /// specifies which type in the cast operator is annotated. A type_argument_index
        /// value of 0 specifies the first (or only) type in the cast operator.
        ///
        /// For an explicit type argument list, the value of the type_argument_index
        /// item specifies which type argument is annotated.
        ///
        /// A type_argument_index value of 0 specifies the first type argument.
        type_argument_index: u8,
    },
}

impl ClassFileItem for TargetInfoType {
    fn read_from_stream<R: Read>(
        s: &mut ClassFileStream<R>,
        cp: Option<&ConstantPool>,
    ) -> error::Result<Self>
    where
        Self: std::marker::Sized,
    {
        match s.read_u1()? {
            0x00 | 0x01 => Ok(Self::TypeParameterTarget {
                type_parameter_index: s.read_u1()?,
            }),
            0x10 => Ok(Self::SupertypeTarget {
                supertype_index: s.read_u2()?,
            }),
            0x11 | 0x12 => Ok(Self::TypeParameterBoundTarget {
                type_parameter_index: s.read_u1()?,
                bound_index: s.read_u1()?,
            }),
            0x13 | 0x14 | 0x15 => Ok(Self::EmptyTarget),
            0x16 => Ok(Self::FormalParameterTarget {
                formal_parameter_index: s.read_u1()?,
            }),
            0x17 => Ok(Self::ThrowsTarget {
                throws_type_index: s.read_u1()?,
            }),
            0x40 | 0x41 => {
                let table_length = s.read_u2()?;
                Ok(Self::LocalVarTarget {
                    table: s.read_sequence(cp, table_length as usize)?,
                })
            }
            0x42 => Ok(Self::CatchTarget {
                exception_table_index: s.read_u2()?,
            }),
            0x43 | 0x44 | 0x45 | 0x46 => Ok(Self::OffsetTarget {
                offset: s.read_u2()?,
            }),
            0x47 | 0x48 | 0x49 | 0x4A | 0x4B => Ok(Self::TypeArgumentTarget {
                offset: s.read_u2()?,
                type_argument_index: s.read_u1()?,
            }),
            v => Err(ClassFileError::UnknownTargetTypeValue(v)),
        }
    }
}

#[derive(Debug)]
/// LocalVarTarget table entry.
pub struct LocalVarTargetTableEntry {
    /// The given local variable has a value at indices into
    /// the code array in the interval [start_pc, start_pc + length),
    /// that is, between start_pc inclusive and start_pc + length exclusive.
    pub pc_range: Range<u16>,
    /// The given local variable must be at index in the
    /// local variable array of the current frame.
    ///
    /// If the local variable at index is of type
    /// double or long, it occupies
    /// both index and index + 1.
    pub index: u16,
}

impl ClassFileItem for LocalVarTargetTableEntry {
    fn read_from_stream<R: Read>(
        s: &mut ClassFileStream<R>,
        cp: Option<&ConstantPool>,
    ) -> error::Result<Self>
    where
        Self: std::marker::Sized,
    {
        Ok(Self {
            pc_range: (s.read_u2()?..s.read_u2()?),
            index: s.read_u2()?,
        })
    }
}

#[derive(Debug)]
/// Parameter annotation.
pub struct ParameterAnnotation {
    /// Each entry in the annotations table represents a single
    /// run-time invisible annotation on the declaration of the
    /// formal parameter corresponding to the
    /// parameter_annotations entry.
    pub annotations: Vec<Annotation>,
}

impl ClassFileItem for ParameterAnnotation {
    fn read_from_stream<R: Read>(
        s: &mut ClassFileStream<R>,
        cp: Option<&ConstantPool>,
    ) -> error::Result<Self>
    where
        Self: std::marker::Sized,
    {
        let num_annotations = s.read_u2()?;
        Ok(Self {
            annotations: s.read_sequence(cp, num_annotations as usize)?,
        })
    }
}

#[derive(Debug)]
/// The annotation structure.
pub struct Annotation {
    pub type_index: u16,
    pub element_value_pairs: Vec<ElementValuePairElement>,
}

impl ClassFileItem for Annotation {
    fn read_from_stream<R: Read>(
        s: &mut ClassFileStream<R>,
        cp: Option<&ConstantPool>,
    ) -> error::Result<Self>
    where
        Self: std::marker::Sized,
    {
        let type_index = s.read_u2()?;
        let num_element_value_pairs = s.read_u2()?;
        Ok(Self {
            type_index,
            element_value_pairs: s.read_sequence(cp, num_element_value_pairs as usize)?,
        })
    }
}

#[derive(Debug)]
/// Element-value-pair element.
pub struct ElementValuePairElement {
    /// The value of the element_name_index item must be a
    /// valid index into the constant_pool table. The
    /// constant_pool entry at that index must be a
    /// CONSTANT_Utf8_info structure (§4.4.7).
    ///
    /// The constant_pool entry denotes the name of the
    /// element of the element-value pair represented
    /// by this element_value_pairs entry.
    pub element_name_index: u16,
    /// The value of the value item represents the
    /// value of the element-value pair represented
    /// by this element_value_pairs entry.
    pub value: ElementValue,
}

impl ClassFileItem for ElementValuePairElement {
    fn read_from_stream<R: Read>(
        s: &mut ClassFileStream<R>,
        cp: Option<&ConstantPool>,
    ) -> error::Result<Self>
    where
        Self: std::marker::Sized,
    {
        Ok(Self {
            element_name_index: s.read_u2()?,
            value: ElementValue::read_from_stream(s, cp)?,
        })
    }
}

/// Element value types.
mod elementvaluetypes {
    use std::io::Read;

    use crate::{
        error::{self, ClassFileError},
        item::{constant_pool::ConstantPool, ClassFileItem},
        stream::ClassFileStream,
    };

    use super::Annotation;

    pub const BYTE: char = 'B';
    pub const CHAR: char = 'C';
    pub const DOUBLE: char = 'D';
    pub const FLOAT: char = 'F';
    pub const INT: char = 'I';
    pub const LONG: char = 'J';
    pub const SHORT: char = 'S';
    pub const BOOLEAN: char = 'Z';
    pub const STRING: char = 's';
    pub const ENUM_TYPE: char = 'e';
    pub const CLASS: char = 'c';
    pub const ANNOTATION_TYPE: char = '@';
    pub const ARRAY_TYPE: char = '[';

    /// Possible element value types.
    pub enum ElementValueType {
        Byte,
        Char,
        Double,
        Float,
        Int,
        Long,
        Short,
        Boolean,
        String,
        Enum,
        Class,
        Annotation,
        Array,
    }

    impl ElementValueType {
        /// Parses the element value type from a character.
        pub fn from_char(c: char) -> error::Result<Self> {
            match c {
                BYTE => Ok(Self::Byte),
                CHAR => Ok(Self::Char),
                DOUBLE => Ok(Self::Double),
                FLOAT => Ok(Self::Float),
                INT => Ok(Self::Int),
                LONG => Ok(Self::Long),
                SHORT => Ok(Self::Short),
                BOOLEAN => Ok(Self::Boolean),
                STRING => Ok(Self::String),
                ENUM_TYPE => Ok(Self::Enum),
                CLASS => Ok(Self::Class),
                ANNOTATION_TYPE => Ok(Self::Annotation),
                ARRAY_TYPE => Ok(Self::Array),
                v => Err(ClassFileError::UnknownElementValueType(v)),
            }
        }
    }

    /// Represents the value of an element-value pair.
    #[derive(Debug)]
    pub enum ElementValue {
        ConstValueIndex {
            /// The const_value_index item denotes either
            /// a primitive constant value or a String
            /// literal as the value of this
            /// element-value pair.
            ///
            /// The value of the const_value_index item must
            /// be a valid index into the constant_pool table.
            /// The constant_pool entry at that index must be
            /// of a type appropriate to the tag item.
            const_value_index: u16,
        },
        /// The enum_const_value item denotes an
        /// enum constant as the value of
        /// this element-value pair.
        EnumConstValue {
            /// The value of the type_name_index item must be a
            /// valid index into the constant_pool table. The
            /// constant_pool entry at that index must be a
            /// CONSTANT_Utf8_info structure (§4.4.7) representing
            /// a field descriptor (§4.3.2).
            ///
            /// The constant_pool entry gives the internal form of
            /// the binary name of the type of the enum constant
            /// represented by this element_value structure (§4.2.1).
            type_name_index: u16,
            /// The value of the const_name_index item must be a
            /// valid index into the constant_pool table.
            ///
            /// The constant_pool entry at that index must be a
            /// CONSTANT_Utf8_info structure (§4.4.7). The
            /// constant_pool entry gives the simple name of
            /// the enum constant represented by this
            /// element_value structure.
            const_name_index: u16,
        },
        ClassInfoIndex {
            /**
            The class_info_index item denotes a class literal as the value of this element-value pair.

            The class_info_index item must be a valid index into the constant_pool table. The constant_pool entry at that index must be a CONSTANT_Utf8_info structure (§4.4.7) representing a return descriptor (§4.3.3). The return descriptor gives the type corresponding to the class literal represented by this element_value structure. Types correspond to class literals as follows:

            For a class literal C.class, where C is the name of a class, interface, or array type, the corresponding type is C. The return descriptor in the constant_pool will be an ObjectType or an ArrayType.

            For a class literal p.class, where p is the name of a primitive type, the corresponding type is p. The return descriptor in the constant_pool will be a BaseType character.

            For a class literal void.class, the corresponding type is void. The return descriptor in the constant_pool will be V.

            For example, the class literal Object.class corresponds to the type Object, so the constant_pool entry is Ljava/lang/Object;, whereas the class literal int.class corresponds to the type int, so the constant_pool entry is I.

            The class literal void.class corresponds to void, so the constant_pool entry is V, whereas the class literal Void.class corresponds to the type Void, so the constant_pool entry is Ljava/lang/Void;.
            **/
            class_info_index: u16,
        },
        AnnotationValue {
            /// The annotation_value item denotes a "nested"
            /// annotation as the value of this element-value pair.
            ///
            /// The value of the annotation_value item is an annotation
            /// structure (§4.7.16) that gives the annotation represented
            /// by this element_value structure.
            annotation_value: Annotation,
        },
        /// The array_value item denotes an array as the value of this element-value pair.
        ArrayValue {
            /// Each value in the values table gives the
            /// corresponding element of the array
            /// represented by this element_value structure.
            values: Vec<ElementValue>,
        },
    }
    impl ClassFileItem for ElementValue {
        fn read_from_stream<R: Read>(
            s: &mut ClassFileStream<R>,
            cp: Option<&ConstantPool>,
        ) -> error::Result<Self>
        where
            Self: Sized,
        {
            match ElementValueType::from_char(s.read_u1()? as char)? {
                ElementValueType::Byte => Ok(Self::ConstValueIndex {
                    const_value_index: s.read_u2()?,
                }),
                ElementValueType::Char => Ok(Self::ConstValueIndex {
                    const_value_index: s.read_u2()?,
                }),
                ElementValueType::Double => Ok(Self::ConstValueIndex {
                    const_value_index: s.read_u2()?,
                }),
                ElementValueType::Float => Ok(Self::ConstValueIndex {
                    const_value_index: s.read_u2()?,
                }),
                ElementValueType::Int => Ok(Self::ConstValueIndex {
                    const_value_index: s.read_u2()?,
                }),
                ElementValueType::Long => Ok(Self::ConstValueIndex {
                    const_value_index: s.read_u2()?,
                }),
                ElementValueType::Short => Ok(Self::ConstValueIndex {
                    const_value_index: s.read_u2()?,
                }),
                ElementValueType::Boolean => Ok(Self::ConstValueIndex {
                    const_value_index: s.read_u2()?,
                }),
                ElementValueType::String => Ok(Self::ConstValueIndex {
                    const_value_index: s.read_u2()?,
                }),
                ElementValueType::Enum => Ok(Self::EnumConstValue {
                    type_name_index: s.read_u2()?,
                    const_name_index: s.read_u2()?,
                }),
                ElementValueType::Class => Ok(Self::ClassInfoIndex {
                    class_info_index: s.read_u2()?,
                }),
                ElementValueType::Annotation => Ok(Self::AnnotationValue {
                    annotation_value: Annotation::read_from_stream(s, cp)?,
                }),
                ElementValueType::Array => {
                    let num_values = s.read_u2()?;
                    Ok(Self::ArrayValue {
                        values: s.read_sequence(cp, num_values as usize)?,
                    })
                }
            }
        }
    }
}

#[derive(Debug)]
/// Local variable type table entry.
pub struct LocalVariableTypeTableEntry {
    /// The given local variable must have a value
    /// at indices into the code array in the interval
    /// [start_pc, start_pc + length), that is, between
    /// start_pc inclusive and start_pc + length exclusive.
    ///
    /// The value of start_pc must be a valid index into the
    /// code array of this Code attribute and must be the
    /// index of the opcode of an instruction.
    ///
    /// The value of start_pc + length must either be a valid
    /// index into the code array of this Code attribute
    /// and be the index of the opcode of an instruction,
    /// or it must be the first index beyond
    /// the end of that code array.
    pub pc_range: Range<u16>,
    /// The value of the name_index item must be a valid index
    /// into the constant_pool table. The constant_pool entry
    /// at that index must contain a CONSTANT_Utf8_info
    /// structure (§4.4.7) representing a valid unqualified
    /// name denoting a local variable (§4.2.2).
    pub name_index: u16,
    /// The value of the signature_index item must be a valid
    /// index into the constant_pool table. The constant_pool
    /// entry at that index must contain a CONSTANT_Utf8_info
    /// structure (§4.4.7) representing a field signature
    /// which encodes the type of a local variable
    /// in the source program (§4.7.9.1).
    pub signature_index: u16,
    /// The given local variable must be at index in
    /// the local variable array of the current frame.
    ///
    /// If the local variable at index is of type
    /// double or long, it occupies both
    /// index and index + 1.
    pub index: u16,
}

impl ClassFileItem for LocalVariableTypeTableEntry {
    fn read_from_stream<R: Read>(
        s: &mut ClassFileStream<R>,
        cp: Option<&ConstantPool>,
    ) -> error::Result<Self>
    where
        Self: std::marker::Sized,
    {
        Ok(Self {
            pc_range: (s.read_u2()?..s.read_u2()?),
            name_index: s.read_u2()?,
            signature_index: s.read_u2()?,
            index: s.read_u2()?,
        })
    }
}

#[derive(Debug)]
pub struct LocalVariableTableEntry {
    /// The given local variable must have a value
    /// at indices into the code array in the interval
    /// [start_pc, start_pc + length), that is, between
    /// start_pc inclusive and start_pc + length exclusive.
    ///
    /// The value of start_pc must be a valid index into the
    /// code array of this Code attribute and must be the
    /// index of the opcode of an instruction.
    ///
    /// The value of start_pc + length must either be a valid
    /// index into the code array of this Code attribute
    /// and be the index of the opcode of an instruction,
    /// or it must be the first index beyond
    /// the end of that code array.
    pub pc_range: Range<u16>,
    /// The value of the name_index item must be a valid index
    /// into the constant_pool table. The constant_pool entry
    /// at that index must contain a CONSTANT_Utf8_info
    /// structure (§4.4.7) representing a valid unqualified
    /// name denoting a local variable (§4.2.2).
    pub name_index: u16,
    /// The value of the descriptor_index item must be a
    /// valid index into the constant_pool table. The
    /// constant_pool entry at that index must contain
    /// a CONSTANT_Utf8_info structure (§4.4.7) representing
    /// a field descriptor which encodes the type of a
    /// local variable in the source program (§4.3.2).
    pub descriptor_index: u16,
    /// The given local variable must be at index in
    /// the local variable array of the current frame.
    ///
    /// If the local variable at index is of type
    /// double or long, it occupies both
    /// index and index + 1.
    pub index: u16,
}

impl ClassFileItem for LocalVariableTableEntry {
    fn read_from_stream<R: Read>(
        s: &mut ClassFileStream<R>,
        cp: Option<&ConstantPool>,
    ) -> error::Result<Self>
    where
        Self: std::marker::Sized,
    {
        Ok(Self {
            pc_range: (s.read_u2()?..s.read_u2()?),
            name_index: s.read_u2()?,
            descriptor_index: s.read_u2()?,
            index: s.read_u2()?,
        })
    }
}

/// An entry in the `line_number_table` table of the `LineNumberTable` attribute.
#[derive(Debug)]
pub struct LineNumberTableEntry {
    /// The value of the start_pc item must indicate
    /// the index into the code array at which the
    /// code for a new line in the original source file begins.
    ///
    /// The value of start_pc must be less than the value
    /// of the code_length item of the Code attribute of
    /// which this LineNumberTable is an attribute.
    pub start_pc: u16,
    /// The value of the line_number item must give the
    /// corresponding line number in the original source file.
    pub line_number: u16,
}

impl ClassFileItem for LineNumberTableEntry {
    fn read_from_stream<R: Read>(
        s: &mut ClassFileStream<R>,
        cp: Option<&ConstantPool>,
    ) -> error::Result<Self>
    where
        Self: std::marker::Sized,
    {
        Ok(Self {
            start_pc: s.read_u2()?,
            line_number: s.read_u2()?,
        })
    }
}

/// An entry in the `classes` array of the `InnerClasses` attribute.
#[derive(Debug)]
pub struct ClassArrayEntry {
    // If a class file has a version number that is 51.0 or above,
    // and has an InnerClasses attribute in its attributes table,
    // then for all entries in the classes array of the InnerClasses attribute,
    // the value of the outer_class_info_index item must be zero
    // if the value of the inner_name_index item is zero.
    /// The value of the inner_class_info_index item must be a
    /// valid index into the constant_pool table.
    ///
    /// The constant_pool entry at that index must be a
    /// CONSTANT_Class_info structure representing C.
    ///
    /// The remaining items in the classes array entry
    /// give information about C.
    pub inner_class_info_index: u16,
    /// If C is not a member of a class or an interface
    /// (that is, if C is a top-level class or interface
    /// (JLS §7.6) or a local class (JLS §14.3) or an
    /// anonymous class (JLS §15.9.5)), the
    /// value of the outer_class_info_index item must be zero.
    ///
    /// Otherwise, the value of the outer_class_info_index item
    /// must be a valid index into the constant_pool table, and
    /// the entry at that index must be a CONSTANT_Class_info
    /// structure representing the class or interface
    /// of which C is a member.
    pub outer_class_info_index: u16,
    /// If C is anonymous (JLS §15.9.5), the value of the
    /// inner_name_index item must be zero.
    ///
    /// Otherwise, the value of the inner_name_index item
    /// must be a valid index into the constant_pool table,
    /// and the entry at that index must be a
    /// CONSTANT_Utf8_info structure (§4.4.7) that represents
    ///  the original simple name of C, as given in the source
    /// code from which this class file was compiled.
    pub inner_name_index: u16,
    /// The value of the inner_class_access_flags item is a mask
    /// of flags used to denote access permissions to and
    /// properties of class or interface C as declared in the
    /// source code from which this class file was compiled.
    ///
    /// It is used by a compiler to recover the original information
    /// when source code is not available.
    pub inner_class_access_flags: ClassAccessFlags,
}

impl ClassFileItem for ClassArrayEntry {
    fn read_from_stream<R: Read>(
        s: &mut ClassFileStream<R>,
        cp: Option<&ConstantPool>,
    ) -> error::Result<Self>
    where
        Self: std::marker::Sized,
    {
        Ok(Self {
            inner_class_info_index: s.read_u2()?,
            outer_class_info_index: s.read_u2()?,
            inner_name_index: s.read_u2()?,
            inner_class_access_flags: ClassAccessFlags::from_bits(s.read_u2()?)
                .ok_or(ClassFileError::BadClassAccessFlags)?,
        })
    }
}

/// An entry in the exception table.
#[derive(Debug)]
pub struct ExceptionTableEntry {
    /// The values of the two items start_pc and end_pc indicate
    /// the ranges in the code array at which the exception
    /// handler is active. The value of start_pc must be a
    /// valid index into the code array of the opcode of
    /// an instruction.
    ///
    /// The value of end_pc either must
    /// be a valid index into the code array of the
    /// opcode of an instruction or must be equal to
    /// code_length, the length of the code array.
    /// The value of start_pc must be less than the
    /// value of end_pc
    /// .
    /// The start_pc is inclusive and end_pc is exclusive; that is,
    /// the exception handler must be active while the program
    /// counter is within the interval [start_pc, end_pc).
    pub pc_range: RangeInclusive<u16>,
    /// The value of the handler_pc item indicates the start of
    /// the exception handler. The value of the item must be a
    /// valid index into the code array and must be the
    /// index of the opcode of an instruction.
    pub handler_pc: u16,
    /**
    If the value of the catch_type item is nonzero, it must be a valid index into the constant_pool table. The constant_pool entry at that index must be a CONSTANT_Class_info structure (§4.4.1) representing a class of exceptions that this exception handler is designated to catch. The exception handler will be called only if the thrown exception is an instance of the given class or one of its subclasses.

    The verifier should check that the class is Throwable or a subclass of Throwable (§4.9.2).

    If the value of the catch_type item is zero, this exception handler is called for all exceptions.

    This is used to implement finally (§3.13).
    **/
    pub catch_type: u16,
}

impl ClassFileItem for ExceptionTableEntry {
    fn read_from_stream<R: Read>(
        s: &mut ClassFileStream<R>,
        cp: Option<&ConstantPool>,
    ) -> error::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            pc_range: (s.read_u2()?..=s.read_u2()?),
            handler_pc: s.read_u2()?,
            catch_type: s.read_u2()?,
        })
    }
}
