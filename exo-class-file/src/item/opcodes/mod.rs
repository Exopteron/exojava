use std::io::Read;

use exo_parser::{error::ParsingError, Lexer};
use fnv::FnvHashMap;

use super::{
    constant_pool::ConstantPoolEntry,
    file::ClassFile,
    ids::{
        class::ClassName,
        field::{FieldDescriptor, FieldType},
        method::MethodDescriptor,
    },
    ClassFileItem, ConstantPool,
};
use crate::{
    error::{self, ClassFileError},
    stream::ClassFileStream,
};

macro_rules! numerical_enum {
    (
        $(#[$inner:ident $($args:tt)*])*
        $name:ident: $vartype:ty {
            $(
                $vident:ident = $val:expr
            ),*
        }
    ) => {
        $(#[$inner $($args)*])*
        #[derive(Clone, Copy, Debug)]
        pub enum $name {
            $(
                $vident
            ),*
        }
        impl ClassFileItem for $name {
            fn read_from_stream<R: Read>(s: &mut ClassFileStream<R>, _cp: Option<&ConstantPool>) -> error::Result<Self> {
                match <$vartype>::read_from_stream(s, _cp)? {
                    $(
                        $val => Ok(Self::$vident),
                    )*
                    v => Err(ClassFileError::UnknownEnumVariant(stringify!($name), v as i32))
                }
            }
        }
    };
}
numerical_enum! {
    /// Types used in `newarray` opcode.
    ArrayTypeCode: u8 {
        T_BOOLEAN = 4,
        T_CHAR = 5,
        T_FLOAT = 6,
        T_DOUBLE = 7,
        T_BYTE = 8,
        T_SHORT = 9,
        T_INT = 10,
        T_LONG = 11
    }
}

#[macro_use]
/// Macro for defining an opcode enum.
/// Automatically implements parsing.
macro_rules! def_opcode {
    (
        $opcodename:ident {
            $(
                $(#[$inner:ident $($args:tt)*])*
                ($code:expr) = $name:ident($($part:ty),*)
            ),*
        }
    ) => {
        #[derive(Debug, Clone)]
        pub enum $opcodename {
            /// Access jump table by key match and jump
            ///
            /// Format: `lookupswitch <0-3 byte pad> defaultbyte1 defaultbyte2 defaultbyte3 defaultbyte4 npairs1 npairs2 npairs3 npairs4 match-offset pairs...
            ///
            /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.lookupswitch)
            lookupswitch(i32, Vec<(i32, i32)>),

            /// Access jump table by index and jump
            ///
            /// Format: `tableswitch <0-3 byte pad> defaultbyte1 defaultbyte2 defaultbyte3 defaultbyte4 lowbyte1 lowbyte2 lowbyte3 lowbyte4 highbyte1 highbyte2 highbyte3 highbyte4 jump offsets...`
            ///
            /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.tableswitch)
            tableswitch(i32, i32, i32, Vec<i32>),

            /// Extend local variable index by additional bytes
            ///
            /// Format: `wide <opcode> indexbyte1 indexbyte2`
            ///
            /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.wide)
            wide_format1(Box<$opcodename>, u16),

            /// Extend local variable index by additional bytes
            ///
            /// Format: `wide iinc indexbyte1 indexbyte2 constbyte1 constbyte2`
            ///
            /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.wide)
            wide_format2(Box<$opcodename>, u16, u16),
            $(
                $(#[$inner $($args)*])*
                $name($($part),*)
            ),*
        }

        impl $opcodename {
            fn read_from_stream<R: Read>(s: &mut ClassFileStream<R>, _cp: Option<&ConstantPool>, current_byte_offset: usize) -> error::Result<(Self, usize)> {
                let start = s.1;
                let v = match s.read_u1()? {
                    $(
                        $code => Self::$name($(<$part>::read_from_stream(s, _cp)?),*),
                    )*
                    0xab => { // lookupswitch special case
                        let pad_count = 4 - (current_byte_offset % 4);
                        s.read_dynamic(pad_count)?;
                        let default = s.read_u4()? as i32;
                        let npairs = s.read_u4()?;

                        let mut pairs = vec![];

                        for _ in 0..npairs {
                            pairs.push((s.read_u4()? as i32, s.read_u4()? as i32));
                        }
                        Self::lookupswitch(default, pairs)
                    },
                    0xaa => { // tableswitch special case
                        let pad_count = 4 - (current_byte_offset % 4);
                        s.read_dynamic(pad_count)?;
                        let default = s.read_u4()? as i32;
                        let low = s.read_u4()? as i32;
                        let high = s.read_u4()? as i32;

                        let mut offsets = vec![];

                        for _ in 0..(high.checked_sub(low + 1).ok_or(ClassFileError::ArithmeticError)?) {
                            offsets.push(s.read_u4()? as i32);
                        }
                        Self::tableswitch(default, low, high, offsets)
                    },
                    0xc4 => { // wide special case
                        let opcode = $opcodename::read_from_stream(s, _cp, current_byte_offset)?;
                        if matches!(opcode.0, $opcodename::iinc( .. )) {
                            Self::wide_format2(Box::new(opcode.0), s.read_u2()?, s.read_u2()?)
                        } else {
                            Self::wide_format1(Box::new(opcode.0), s.read_u2()?)
                        }
                    }
                    v => return Err(ClassFileError::UnknownOpcodeError(v))
                };
                Ok((v, s.1.checked_sub(start).ok_or(ClassFileError::ArithmeticError)?))
            }
        }

        // impl ClassFileItem for $opcodename {
        //     fn read_from_stream<R: Read>(
        //         s: &mut ClassFileStream<R>,
        //         _cp: Option<&ConstantPool>,
        //     ) -> error::Result<Self>
        //     where
        //         Self: Sized,
        //     {
        //         match s.read_u1()? {
        //             $(
        //                 $code => Ok(Self::$name($(<$part>::read_from_stream(s, _cp)?),*)),
        //             )*
        //             v => Err(ClassFileError::UnknownOpcodeError(v))
        //         }
        //     }
        // }
    };
}

/// A list of JVM instructions.
#[derive(Debug, Clone)]
pub struct InstructionList {
    pub opcodes: Vec<VMOpcode>,
    pub byte_to_code: FnvHashMap<usize, usize>,
    pub code_to_byte: FnvHashMap<usize, usize>
}

/// Possible errors to come from code verification.
#[derive(Debug)]
pub enum CodeVerificationError {
    /// Returned when a branch location is out of bounds.
    BranchLocOutOfBounds,

    /// Returned when the `low` value of a tableswitch is greater than its `high`.
    TableSwitchLowGtHigh,

    /// Returned when an invalid opcode is inside a `wide` instruction.
    BadWideOp,

    /// Bad sorting of lookup switch
    LookupSwitchBadSort,

    /// Bad constant pool index
    BadConstantPoolIndex,

    /// Bad constant pool type
    BadConstantPoolType,

    /// Invokeinterfaze zero byte not zero
    InvokeInterfaceNotZero,

    /// Bad parse
    BadParse(ParsingError),

    /// Generic class file error
    ClassFileError(ClassFileError),

    /// Wrong constant type
    WrongConstantType,

    /// Bad count in invokeinterface
    InvokeInterfaceBadCount,

    /// Bad method name
    BadMethodName,

    /// `new` not referencing a class
    NewNotRefClass,

    /// Bad `anewarray`
    BadANewArray,

    /// Bad `multianewarray`
    BadMultiANewArray,

    /// Local index out of range
    LocalIndexOutOfRange
}

/// Check that an entry in the constant pool matches some pattern `p`.
macro_rules! check_constant_pool {
    ($v:expr, $cp:expr, $p:pat) => {{
        if $v as usize > $cp.entries.len() {
            return Err(CodeVerificationError::BadConstantPoolIndex);
        }

        let entry = $cp.get_constant($v as usize).map_err(CodeVerificationError::ClassFileError)?;
        match entry {
            $p => Ok(()),
            _ => Err(CodeVerificationError::BadConstantPoolType),
        }
    }};
}

macro_rules! get_name_and_type {
    ($index:expr, $cp:expr) => {{
        let (name_index, descriptor_index) = if let ConstantPoolEntry::NameAndType {
            name_index,
            descriptor_index,
        } = $cp.get_constant($index as usize).map_err(CodeVerificationError::ClassFileError)?
        {
            (*name_index, *descriptor_index)
        } else {
            return Err(CodeVerificationError::WrongConstantType);
        };
        (
            $cp.get_utf8_constant(name_index as usize)
                .map_err(CodeVerificationError::ClassFileError)?,
            $cp.get_utf8_constant(descriptor_index as usize)
                .map_err(CodeVerificationError::ClassFileError)?,
        )
    }};
}

macro_rules! parse_str {
    ($str:expr, $token:ty) => {{
        let lexer = Lexer::new();
        let mut stream = Lexer::stream(lexer, $str);
        stream
            .token::<$token>()
            .map_err(|(v, _)| CodeVerificationError::BadParse(v))
    }};
}
macro_rules! get_class {
    ($index:expr, $cp:expr, $parsety:ty) => {{
        if let ConstantPoolEntry::Class { name_index } = $cp.get_constant($index as usize).map_err(CodeVerificationError::ClassFileError)? {
            let str = $cp
                .get_utf8_constant(*name_index as usize)
                .map_err(CodeVerificationError::ClassFileError)?;
            parse_str!(str.to_string(), $parsety)
        } else {
            Err(CodeVerificationError::WrongConstantType)
        }
    }};
}

impl InstructionList {
    /// Verify code based on the constraints detailed
    /// in the [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-4.html#jvms-4.9.1)
    pub fn static_verify(
        &self,
        file: &ClassFile,
        max_locals: usize,
    ) -> std::result::Result<(), CodeVerificationError> {
        for ins in self.opcodes.iter().enumerate() {
            self.static_verify_inst(file, ins.1, ins.0, max_locals, None)?;
        }
        Ok(())
    }

    fn static_verify_inst(
        &self,
        file: &ClassFile,
        inst: &VMOpcode,
        position: usize,
        max_locals: usize,
        wide_index: Option<u16>,
    ) -> std::result::Result<(), CodeVerificationError> {
        if max_locals == 0 {
            return Err(CodeVerificationError::ClassFileError(ClassFileError::ArithmeticError));
        }
        match inst {
            VMOpcode::goto(v)
            | VMOpcode::ifeq(v)
            | VMOpcode::ifne(v)
            | VMOpcode::ifle(v)
            | VMOpcode::iflt(v)
            | VMOpcode::ifge(v)
            | VMOpcode::ifgt(v)
            | VMOpcode::ifnull(v)
            | VMOpcode::ifnonnull(v)
            | VMOpcode::if_icmpeq(v)
            | VMOpcode::if_icmpne(v)
            | VMOpcode::if_icmple(v)
            | VMOpcode::if_icmplt(v)
            | VMOpcode::if_icmpge(v)
            | VMOpcode::if_icmpgt(v)
            | VMOpcode::if_acmpeq(v)
            | VMOpcode::if_acmpne(v) => {
                if *v as usize > self.opcodes.len() {
                    return Err(CodeVerificationError::BranchLocOutOfBounds);
                }
            }
            VMOpcode::goto_w(v) => {
                if *v as usize > self.opcodes.len() {
                    return Err(CodeVerificationError::BranchLocOutOfBounds);
                }
            }
            VMOpcode::tableswitch(default, low, high, jump_offsets) => {
                if ((position as isize) + (*default as isize)) as usize > self.opcodes.len() {
                    return Err(CodeVerificationError::BranchLocOutOfBounds);
                }
                if *low > *high {
                    return Err(CodeVerificationError::TableSwitchLowGtHigh);
                }
                for off in jump_offsets.iter().copied() {
                    let pos = ((position as isize) + (off as isize)) as usize;
                    if pos > self.opcodes.len() {
                        return Err(CodeVerificationError::BranchLocOutOfBounds);
                    }
                }
            }
            VMOpcode::wide_format1(op, index) => {
                match &**op {
                    VMOpcode::iload(_)
                    | VMOpcode::fload(_)
                    | VMOpcode::aload(_)
                    | VMOpcode::lload(_)
                    | VMOpcode::dload(_)
                    | VMOpcode::istore(_)
                    | VMOpcode::fstore(_)
                    | VMOpcode::astore(_)
                    | VMOpcode::lstore(_)
                    | VMOpcode::dstore(_)
                    | VMOpcode::ret(_) => (),
                    _ => return Err(CodeVerificationError::BadWideOp),
                }
                self.static_verify_inst(file, op, position, max_locals, Some(*index))?;
            }
            VMOpcode::wide_format2(iinc, index, constant) => {}
            VMOpcode::lookupswitch(default, match_offset_pairs) => {
                if ((position as isize) + (*default as isize)) as usize > self.opcodes.len() {
                    return Err(CodeVerificationError::BranchLocOutOfBounds);
                }

                let mut last = i32::MIN;
                for (v, offset) in match_offset_pairs.iter().copied() {
                    if last > v {
                        return Err(CodeVerificationError::LookupSwitchBadSort);
                    }
                    last = v;
                    if ((position as isize) + (offset as isize)) as usize > self.opcodes.len() {
                        return Err(CodeVerificationError::BranchLocOutOfBounds);
                    }
                }
            }
            VMOpcode::ldc(v) => {
                self.static_verify_inst(file, &&VMOpcode::ldc_w(*v as u16), position, max_locals, wide_index)?;
            }
            VMOpcode::ldc_w(v) => {
                check_constant_pool!(
                    *v,
                    file.constant_pool,
                    ConstantPoolEntry::Integer { .. }
                        | ConstantPoolEntry::Float { .. }
                        | ConstantPoolEntry::String { .. }
                        | ConstantPoolEntry::Class { .. }
                        | ConstantPoolEntry::MethodType { .. }
                        | ConstantPoolEntry::MethodHandle { .. }
                )?;
            }
            VMOpcode::ldc2_w(v) => {
                check_constant_pool!(
                    *v,
                    file.constant_pool,
                    (ConstantPoolEntry::Long { .. } | ConstantPoolEntry::Double { .. })
                )?;
            }
            VMOpcode::getfield(v)
            | VMOpcode::putfield(v)
            | VMOpcode::getstatic(v)
            | VMOpcode::putstatic(v) => {
                check_constant_pool!(*v, file.constant_pool, ConstantPoolEntry::Fieldref { .. })?;
            }
            VMOpcode::invokevirtual(v) => {
                check_constant_pool!(*v, file.constant_pool, ConstantPoolEntry::Methodref { .. })?;
                let (name, descriptor) = get_name_and_type!(*v, file.constant_pool);
                if name.starts_with("<") {
                    return Err(CodeVerificationError::BadMethodName);
                }
            }
            VMOpcode::invokespecial(v) | VMOpcode::invokestatic(v) => {
                check_constant_pool!(
                    *v,
                    file.constant_pool,
                    ConstantPoolEntry::Methodref { .. }
                        | ConstantPoolEntry::InterfaceMethodref { .. }
                )?;

                let (name, descriptor) = get_name_and_type!(*v, file.constant_pool);
                if let VMOpcode::invokespecial(_) = inst {
                    if name.starts_with("<") && !(name == "<init>") {
                        return Err(CodeVerificationError::BadMethodName);
                    }
                } else {
                    if name.starts_with("<") {
                        return Err(CodeVerificationError::BadMethodName);
                    }
                }
            }
            VMOpcode::invokeinterface(index, count, zero) => {
                check_constant_pool!(
                    *index,
                    file.constant_pool,
                    ConstantPoolEntry::InterfaceMethodref { .. }
                )?;
                if *zero != 0 {
                    return Err(CodeVerificationError::InvokeInterfaceNotZero);
                };
                let (name, descriptor) = get_name_and_type!(*index, file.constant_pool);

                let lexer = Lexer::new();
                let mut stream = Lexer::stream(lexer, descriptor.to_string());

                let mut v = stream
                    .token::<MethodDescriptor>()
                    .map_err(|(v, _)| CodeVerificationError::BadParse(v))?;

                if *count as usize != v.parameters.len() {
                    return Err(CodeVerificationError::InvokeInterfaceBadCount);
                }

                if name.starts_with("<") {
                    return Err(CodeVerificationError::BadMethodName);
                }
            }
            VMOpcode::instanceof(v)
            | VMOpcode::checkcast(v)
            | VMOpcode::new(v)
            | VMOpcode::anewarray(v)
            | VMOpcode::multianewarray(v, _) => {
                check_constant_pool!(*v, file.constant_pool, ConstantPoolEntry::Class { .. })?;
                match inst {
                    VMOpcode::new(v) => {
                        if get_class!(*v, file.constant_pool, ClassName).is_err() {
                            return Err(CodeVerificationError::NewNotRefClass);
                        }
                    }
                    VMOpcode::anewarray(v) => {
                        if let Ok(v) = get_class!(*v, file.constant_pool, ClassName) {
                        } else if let Ok(v) = get_class!(*v, file.constant_pool, FieldDescriptor) {
                            let mut dimensions = 1;
                            let mut v = &v.token;
                            loop {
                                if let FieldType::ArrayType(b) = v {
                                    v = &b.0;
                                    dimensions += 1;
                                } else {
                                    break;
                                }
                            }
                            if dimensions > 255 {
                                return Err(CodeVerificationError::BadANewArray);
                            }
                        } else {
                            return Err(CodeVerificationError::BadANewArray);
                        }
                    }
                    VMOpcode::multianewarray(index, dimensions) => {
                        if *dimensions == 0 {
                            return Err(CodeVerificationError::BadMultiANewArray);
                        }
                    }
                    VMOpcode::wide_format2(_, index, constant) => {
                        if *index as usize > (max_locals - 1) {
                            return Err(CodeVerificationError::LocalIndexOutOfRange);
                        }
                    }
                    VMOpcode::iload(v) | VMOpcode::fload(v) | VMOpcode::aload(v) | VMOpcode::istore(v) | VMOpcode::fstore(v) | VMOpcode::astore(v) | VMOpcode::iinc(v, _) | VMOpcode::ret(v) => {
                        if *v as usize > (max_locals - 1) {
                            return Err(CodeVerificationError::LocalIndexOutOfRange);
                        }
                    }
                    // TODO rest of static assertions
                    _ => (),
                }
            },

            _ => (),
        }
        Ok(())
    }
}

impl ClassFileItem for InstructionList {
    fn read_from_stream<R: Read>(
        s: &mut ClassFileStream<R>,
        cp: Option<&ConstantPool>,
    ) -> error::Result<Self>
    where
        Self: Sized,
    {
        let mut off = 0;
        let mut list = vec![];
        let mut byte_to_code = FnvHashMap::default();
        let mut code_to_byte = FnvHashMap::default();
        while let Ok(c) = VMOpcode::read_from_stream(s, cp, off) {
            code_to_byte.insert(list.len(), off);
            for i in off..off + c.1 {
                println!("I {:?} is {:?}", i, list.len());
                byte_to_code.insert(i, list.len());
            }
            off += c.1;
            list.push(c.0);
        }
        Ok(Self { opcodes: list, byte_to_code, code_to_byte })
    }
}

def_opcode! {
    VMOpcode {
        /// Load `reference` from array
        ///
        /// Format: `aaload`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.aaload)
        (0x32) = aaload(),

        /// Store into `reference` array
        ///
        /// Format: `aastore`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.aastore)
        (0x53) = aastore(),

        /// Push `null`
        ///
        /// Format: `aconst_null`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.aconst_null)
        (0x1) = aconst_null(),

        /// Load `reference` from local variable
        ///
        /// Format: `aload index`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.aload)
        (0x19) = aload(u8),

        /// Load `reference` from local variable
        ///
        /// Format: `aload_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.aload_n)
        (0x2a) = aload_0(),

        /// Load `reference` from local variable
        ///
        /// Format: `aload_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.aload_n)
        (0x2b) = aload_1(),

        /// Load `reference` from local variable
        ///
        /// Format: `aload_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.aload_n)
        (0x2c) = aload_2(),

        /// Load `reference` from local variable
        ///
        /// Format: `aload_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.aload_n)
        (0x2d) = aload_3(),

        /// Create new array of `reference`
        ///
        /// Format: `anewarray indexbyte1 indexbyte2`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.anewarray)
        (0xbd) = anewarray(u16),

        /// Return `reference` from method
        ///
        /// Format: `areturn`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.areturn)
        (0xb0) = areturn(),

        /// Get length of array
        ///
        /// Format: `arraylength`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.arraylength)
        (0xbe) = arraylength(),

        /// Store `reference` into local variable
        ///
        /// Format: `astore index`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.astore)
        (0x3a) = astore(u8),

        /// Store `reference` into local variable
        ///
        /// Format: `astore_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.astore_n)
        (0x4b) = astore_0(),

        /// Store `reference` into local variable
        ///
        /// Format: `astore_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.astore_n)
        (0x4c) = astore_1(),

        /// Store `reference` into local variable
        ///
        /// Format: `astore_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.astore_n)
        (0x4d) = astore_2(),

        /// Store `reference` into local variable
        ///
        /// Format: `astore_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.astore_n)
        (0x4e) = astore_3(),

        /// Throw exception or error
        ///
        /// Format: `athrow`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.athrow)
        (0xbf) = athrow(),

        /// Load `byte` or `boolean` from array
        ///
        /// Format: `baload`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.baload)
        (0x33) = baload(),

        /// Store into `byte` or `boolean` array
        ///
        /// Format: `bastore`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.bastore)
        (0x54) = bastore(),

        /// Push `byte`
        ///
        /// Format: `bipush byte`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.bipush)
        (0x10) = bipush(u8),

        /// Load `char` from array
        ///
        /// Format: `caload`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.caload)
        (0x34) = caload(),

        /// Store into `char` array
        ///
        /// Format: `castore`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.castore)
        (0x55) = castore(),

        /// Check whether object is of given type
        ///
        /// Format: `checkcast indexbyte1 indexbyte2`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.checkcast)
        (0xc0) = checkcast(u16),

        /// Convert `double` to `float`
        ///
        /// Format: `d2f`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.d2f)
        (0x90) = d2f(),

        /// Convert `double` to `int`
        ///
        /// Format: `d2i`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.d2i)
        (0x8e) = d2i(),

        /// Convert `double` to `long`
        ///
        /// Format: `d2l`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.d2l)
        (0x8f) = d2l(),

        /// Add `double`
        ///
        /// Format: `dadd`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.dadd)
        (0x63) = dadd(),

        /// Load `double` from array
        ///
        /// Format: `daload`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.daload)
        (0x31) = daload(),

        /// Store into `double` array
        ///
        /// Format: `dastore`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.dastore)
        (0x52) = dastore(),

        /// Compare `double`
        ///
        /// Format: `dcmp<op>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.dcmp_op)
        (0x98) = dcmpg(),

        /// Compare `double`
        ///
        /// Format: `dcmp<op>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.dcmp_op)
        (0x97) = dcmpl(),


        /// Push `double`
        ///
        /// Format: `dconst_<d>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.dconst_d)
        (0xe) = dconst_0(),

        /// Push `double`
        ///
        /// Format: `dconst_<d>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.dconst_d)
        (0xf) = dconst_1(),

        /// Divide `double`
        ///
        /// Format: `ddiv`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.ddiv)
        (0x6f) = ddiv(),

        /// Load `double` from local variable
        ///
        /// Format: `dload`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.dload)
        (0x18) = dload(u8),

        /// Load `double` from local variable
        ///
        /// Format: `dload_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.dload_n)
        (0x26) = dload_0(),

        /// Load `double` from local variable
        ///
        /// Format: `dload_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.dload_n)
        (0x27) = dload_1(),

        /// Load `double` from local variable
        ///
        /// Format: `dload_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.dload_n)
        (0x28) = dload_2(),

        /// Load `double` from local variable
        ///
        /// Format: `dload_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.dload_n)
        (0x29) = dload_3(),

        /// Multiply `double`
        ///
        /// Format: `dmul`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.dmul)
        (0x6b) = dmul(),

        /// Negate `double`
        ///
        /// Format: `dneg`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.dneg)
        (0x77) = dneg(),

        /// Remainder `double`
        ///
        /// Format: `drem`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.drem)
        (0x73) = drem(),

        /// Return `double` from method
        ///
        /// Format: `dreturn`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.dreturn)
        (0xaf) = dreturn(),

        /// Store `double` into local variable
        ///
        /// Format: `dstore index`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.dstore)
        (0x39) = dstore(u8),

        /// Store `double` into local variable
        ///
        /// Format: `dstore_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.dstore_n)
        (0x47) = dstore_0(),

        /// Store `double` into local variable
        ///
        /// Format: `dstore_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.dstore_n)
        (0x48) = dstore_1(),

        /// Store `double` into local variable
        ///
        /// Format: `dstore_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.dstore_n)
        (0x49) = dstore_2(),

        /// Store `double` into local variable
        ///
        /// Format: `dstore_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.dstore_n)
        (0x4a) = dstore_3(),

        /// Subtract `double`
        ///
        /// Format: `dsub`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.dsub)
        (0x67) = dsub(),

        /// Duplicate the top operand stack value
        ///
        /// Format: `dup`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.dup)
        (0x59) = dup(),

        /// Duplicate the top operand stack value and insert two values down
        ///
        /// Format: `dup_x1`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.dup_x1)
        (0x5a) = dup_x1(),

        /// Duplicate the top operand stack value and insert two or three values down
        ///
        /// Format: `dup_x2`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.dup_x2)
        (0x5b) = dup_x2(),

        /// Duplicate the top one or two operand stack values
        ///
        /// Format: `dup2`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.dup2)
        (0x5c) = dup2(),

        /// Duplicate the top one or two operand stack values and insert two or three values down
        ///
        /// Format: `dup2_x1`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.dup2_x1)
        (0x5d) = dup2_x1(),

        /// Duplicate the top one or two operand stack values and insert two, three, or four values down
        ///
        /// Format: `dup2_x2`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.dup2_x2)
        (0x5e) = dup2_x2(),

        /// Convert `float` to `double`
        ///
        /// Format: `f2d`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.f2d)
        (0x8d) = f2d(),

        /// Convert `float` to `int`
        ///
        /// Format: `f2i`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.f2i)
        (0x8b) = f2i(),

        /// Convert `float` to `long`
        ///
        /// Format: `f2l`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.f2l)
        (0x8c) = f2l(),

        /// Add `float`
        ///
        /// Format: `fadd`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.fadd)
        (0x62) = fadd(),

        /// Load `float` from array
        ///
        /// Format: `faload`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.faload)
        (0x30) = faload(),

        /// Store into `float` array
        ///
        /// Format: `fastore`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.fastore)
        (0x51) = fastore(),

        /// Compare `float`
        ///
        /// Format: `fcmp<op>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.fcmp_op)
        (0x96) = fcmpg(),

        /// Compare `float`
        ///
        /// Format: `fcmp<op>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.fcmp_op)
        (0x95) = fcmpl(),

        /// Push `float`
        ///
        /// Format: `fconst_<f>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.fconst_f)
        (0xb) = fconst_0(),

        /// Push `float`
        ///
        /// Format: `fconst_<f>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.fconst_f)
        (0xc) = fconst_1(),

        /// Push `float`
        ///
        /// Format: `fconst_<f>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.fconst_f)
        (0xd) = fconst_2(),

        /// Divide `float`
        ///
        /// Format: `fdiv`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.fdiv)
        (0x6e) = fdiv(),

        /// Load `float` from local variable
        ///
        /// Format: `fload index`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.fload)
        (0x17) = fload(u8),

        /// Load `float` from local variable
        ///
        /// Format: `fload_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.fload_n)
        (0x22) = fload_0(),

        /// Load `float` from local variable
        ///
        /// Format: `fload_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.fload_n)
        (0x23) = fload_1(),

        /// Load `float` from local variable
        ///
        /// Format: `fload_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.fload_n)
        (0x24) = fload_2(),

        /// Load `float` from local variable
        ///
        /// Format: `fload_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.fload_n)
        (0x25) = fload_3(),

        /// Multiply `float`
        ///
        /// Format: `fmul`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.fmul)
        (0x6a) = fmul(),

        /// Negate `float`
        ///
        /// Format: `fneg`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.fneg)
        (0x76) = fneg(),

        /// Remainder `float`
        ///
        /// Format: `frem`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.frem)
        (0x72) = frem(),

        /// Return `float` from method
        ///
        /// Format: `freturn`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.freturn)
        (0xae) = freturn(),

        /// Store `float` into local variable
        ///
        /// Format: `fstore index`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.fstore)
        (0x38) = fstore(u8),

        /// Store `float` into local variable
        ///
        /// Format: `fstore_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.fstore_n)
        (0x43) = fstore_0(),

        /// Store `float` into local variable
        ///
        /// Format: `fstore_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.fstore_n)
        (0x44) = fstore_1(),

        /// Store `float` into local variable
        ///
        /// Format: `fstore_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.fstore_n)
        (0x45) = fstore_2(),

        /// Store `float` into local variable
        ///
        /// Format: `fstore_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.fstore_n)
        (0x46) = fstore_3(),

        /// Subtract `float`
        ///
        /// Format: `fsub`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.fsub)
        (0x66) = fsub(),

        /// Fetch field from object
        ///
        /// Format: `getfield indexbyte1 indexbyte2`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.getfield)
        (0xb4) = getfield(u16),

        /// Get `static` field from class
        ///
        /// Format: `getstatic indexbyte1 indexbyte2`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.getstatic)
        (0xb2) = getstatic(u16),

        /// Branch always
        ///
        /// Format: `goto branchbyte1 branchbyte2`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.goto)
        (0xa7) = goto(i16),

        /// Branch always (wide index)
        ///
        /// Format: `goto branchbyte1 branchbyte2 branchbyte3 branchbyte4`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.goto_w)
        (0xc8) = goto_w(u32),

        /// Convert `int` to `byte`
        ///
        /// Format: `i2b`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.i2b)
        (0x91) = i2b(),

        /// Convert `int` to `char`
        ///
        /// Format: `i2c`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.i2c)
        (0x92) = i2c(),

        /// Convert `int` to `double`
        ///
        /// Format: `i2d`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.i2d)
        (0x87) = i2d(),

        /// Convert `int` to `float`
        ///
        /// Format: `i2f`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.i2f)
        (0x86) = i2f(),

        /// Convert `int` to `long`
        ///
        /// Format: `i2l`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.i2l)
        (0x85) = i2l(),

        /// Convert `int` to `short`
        ///
        /// Format: `i2s`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.i2s)
        (0x93) = i2s(),

        /// Add `int`
        ///
        /// Format: `iadd`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.iadd)
        (0x60) = iadd(),

        /// Load `int` from array
        ///
        /// Format: `iaload`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.iaload)
        (0x2e) = iaload(),

        /// Boolean AND `int`
        ///
        /// Format: `iand`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.iand)
        (0x7e) = iand(),

        /// Store into `int` array
        ///
        /// Format: `iastore`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.iastore)
        (0x4f) = iastore(),

        /// Push `int` constant
        ///
        /// Format: `iconst_<i>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.iconst_i)
        (0x2) = iconst_m1(),

        /// Push `int` constant
        ///
        /// Format: `iconst_<i>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.iconst_i)
        (0x3) = iconst_0(),

        /// Push `int` constant
        ///
        /// Format: `iconst_<i>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.iconst_i)
        (0x4) = iconst_1(),

        /// Push `int` constant
        ///
        /// Format: `iconst_<i>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.iconst_i)
        (0x5) = iconst_2(),

        /// Push `int` constant
        ///
        /// Format: `iconst_<i>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.iconst_i)
        (0x6) = iconst_3(),

        /// Push `int` constant
        ///
        /// Format: `iconst_<i>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.iconst_i)
        (0x7) = iconst_4(),

        /// Push `int` constant
        ///
        /// Format: `iconst_<i>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.iconst_i)
        (0x8) = iconst_5(),

        /// Divide `int`
        ///
        /// Format: `idiv`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.idiv)
        (0x6c) = idiv(),

        /// Branch if `reference` comparison succeeds
        ///
        /// Format: `if_acmp<cond> branchbyte1 branchbyte2`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.if_acmp_cond)
        (0xa5) = if_acmpeq(i16),

        /// Branch if `reference` comparison succeeds
        ///
        /// Format: `if_acmp<cond> branchbyte1 branchbyte2`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.if_acmp_cond)
        (0xa6) = if_acmpne(i16),

        /// Branch if `int` comparison succeeds
        ///
        /// Format: `if_icmp<cond> branchbyte1 branchbyte2`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.if_icmp_cond)
        (0x9f) = if_icmpeq(i16),

        /// Branch if `int` comparison succeeds
        ///
        /// Format: `if_icmp<cond> branchbyte1 branchbyte2`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.if_icmp_cond)
        (0xa0) = if_icmpne(i16),

        /// Branch if `int` comparison succeeds
        ///
        /// Format: `if_icmp<cond> branchbyte1 branchbyte2`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.if_icmp_cond)
        (0xa1) = if_icmplt(i16),

        /// Branch if `int` comparison succeeds
        ///
        /// Format: `if_icmp<cond> branchbyte1 branchbyte2`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.if_icmp_cond)
        (0xa2) = if_icmpge(i16),

        /// Branch if `int` comparison succeeds
        ///
        /// Format: `if_icmp<cond> branchbyte1 branchbyte2`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.if_icmp_cond)
        (0xa3) = if_icmpgt(i16),

        /// Branch if `int` comparison succeeds
        ///
        /// Format: `if_icmp<cond> branchbyte1 branchbyte2`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.if_icmp_cond)
        (0xa4) = if_icmple(i16),

        /// Branch if `int` comparison with zero succeeds
        ///
        /// Format: `if<cond> branchbyte1 branchbyte2`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.if_cond)
        (0x99) = ifeq(i16),

        /// Branch if `int` comparison with zero succeeds
        ///
        /// Format: `if<cond> branchbyte1 branchbyte2`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.if_cond)
        (0x9a) = ifne(i16),

        /// Branch if `int` comparison with zero succeeds
        ///
        /// Format: `if<cond> branchbyte1 branchbyte2`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.if_cond)
        (0x9b) = iflt(i16),

        /// Branch if `int` comparison with zero succeeds
        ///
        /// Format: `if<cond> branchbyte1 branchbyte2`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.if_cond)
        (0x9c) = ifge(i16),

        /// Branch if `int` comparison with zero succeeds
        ///
        /// Format: `if<cond> branchbyte1 branchbyte2`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.if_cond)
        (0x9d) = ifgt(i16),

        /// Branch if `int` comparison with zero succeeds
        ///
        /// Format: `if<cond> branchbyte1 branchbyte2`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.if_cond)
        (0x9e) = ifle(i16),

        /// Branch if `reference` not `null`
        ///
        /// Format: `ifnonnull branchbyte1 branchbyte2`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.ifnonnull)
        (0xc7) = ifnonnull(i16),

        /// Branch if `reference` is `null`
        ///
        /// Format: `ifnull branchbyte1 branchbyte2`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.ifnull)
        (0xc6) = ifnull(i16),

        /// Increment local variable by constant
        ///
        /// Format: `iinc index const`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.iinc)
        (0x84) = iinc(u8, u8),

        /// Load `int` from local variable
        ///
        /// Format: `iload index`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.iload)
        (0x15) = iload(u8),

        /// Load `int` from local variable
        ///
        /// Format: `iload_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.iload_n)
        (0x1a) = iload_0(),

        /// Load `int` from local variable
        ///
        /// Format: `iload_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.iload_n)
        (0x1b) = iload_1(),

        /// Load `int` from local variable
        ///
        /// Format: `iload_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.iload_n)
        (0x1c) = iload_2(),

        /// Load `int` from local variable
        ///
        /// Format: `iload_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.iload_n)
        (0x1d) = iload_3(),

        /// Multiply `int`
        ///
        /// Format: `imul`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.imul)
        (0x68) = imul(),

        /// Negate `int`
        ///
        /// Format: `ineg`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.ineg)
        (0x74) = ineg(),

        /// Determine if object is of given type
        ///
        /// Format: `instanceof indexbyte1 indexbyte2`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.instanceof)
        (0xc1) = instanceof(u16),

        /// Invoke dynamic method
        ///
        /// Format: `invokedynamic indexbyte1 indexbyte2 0 0`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.invokedynamic)
        (0xba) = invokedynamic(u16, u16),

        /// Invoke interface method
        ///
        /// Format: `invokeinterface indexbyte1 indexbyte2 count 0`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.invokeinterface)
        (0xb9) = invokeinterface(u16, u8, u8),

        /// Invoke instance method; special handling for superclass, private, and instance initialization method invocations
        ///
        /// Format: `invokespecial indexbyte1 indexbyte2`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.invokespecial)
        (0xb7) = invokespecial(u16),

        /// Invoke a class (`static`) method
        ///
        /// Format: `invokestatic indexbyte1 indexbyte2`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.invokestatic)
        (0xb8) = invokestatic(u16),

        /// Invoke instance method; dispatch based on class
        ///
        /// Format: `invokevirtual indexbyte1 indexbyte2`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.invokevirtual)
        (0xb6) = invokevirtual(u16),

        /// Boolean OR `int`
        ///
        /// Format: `ior`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.ior)
        (0x80) = ior(),

        /// Remainder `int`
        ///
        /// Format: `irem`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.irem)
        (0x70) = irem(),

        /// Return `int` from method
        ///
        /// Format: `ireturn`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.ireturn)
        (0xac) = ireturn(),

        /// Shift left `int`
        ///
        /// Format: `ishl`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.ishl)
        (0x78) = ishl(),

        /// Arithmetic shift right `int`
        ///
        /// Format: `ishr`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.ishr)
        (0x7a) = ishr(),

        /// Store `int` into local variable
        ///
        /// Format: `istore index`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.istore)
        (0x36) = istore(u8),

        /// Store `int` into local variable
        ///
        /// Format: `istore_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.istore_n)
        (0x3b) = istore_0(),

        /// Store `int` into local variable
        ///
        /// Format: `istore_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.istore_n)
        (0x3c) = istore_1(),

        /// Store `int` into local variable
        ///
        /// Format: `istore_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.istore_n)
        (0x3d) = istore_2(),

        /// Store `int` into local variable
        ///
        /// Format: `istore_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.istore_n)
        (0x3e) = istore_3(),

        /// Subtract `int`
        ///
        /// Format: `isub`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.isub)
        (0x64) = isub(),

        /// Logical shift right `int`
        ///
        /// Format: `iushr`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.iushr)
        (0x7c) = iushr(),

        /// Boolean XOR `int`
        ///
        /// Format: `ixor`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.ixor)
        (0x82) = ixor(),

        /// Convert `long` to `double`
        ///
        /// Format: `l2d`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.l2d)
        (0x8a) = l2d(),

        /// Convert `long` to `float`
        ///
        /// Format: `l2f`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.l2f)
        (0x89) = l2f(),

        /// Convert `long` to `int`
        ///
        /// Format: `l2i`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.l2i)
        (0x88) = l2i(),

        /// Add `long`
        ///
        /// Format: `ladd`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.ladd)
        (0x61) = ladd(),

        /// Load `long` from array
        ///
        /// Format: `laload`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.laload)
        (0x2f) = laload(),

        /// Boolean AND `long`
        ///
        /// Format: `land`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.land)
        (0x7f) = land(),

        /// Store into `long` array
        ///
        /// Format: `lastore`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.lastore)
        (0x50) = lastore(),

        /// Compare `long`
        ///
        /// Format: `lcmp`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.lcmp)
        (0x94) = lcmp(),

        /// Push `long` constant
        ///
        /// Format: `lconst_<l>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.lconst_l)
        (0x9) = lconst_0(),

        /// Push `long` constant
        ///
        /// Format: `lconst_<l>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.lconst_l)
        (0xa) = lconst_1(),

        /// Push item from run-time constant pool
        ///
        /// Format: `ldc index`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.ldc)
        (0x12) = ldc(u8),

        /// Push item from run-time constant pool (wide index)
        ///
        /// Format: `ldc_w indexbyte1 indexbyte2`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.ldc_w)
        (0x13) = ldc_w(u16),

        /// Push `long` or `double` from run-time constant pool (wide index)
        ///
        /// Format: `ldc2_w indexbyte1 indexbyte2`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.ldc2_w)
        (0x14) = ldc2_w(u16),

        /// Divide `long`
        ///
        /// Format: `ldiv`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.ldiv)
        (0x6d) = ldiv(),

        /// Load `long` from local variable
        ///
        /// Format: `lload index`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.lload)
        (0x16) = lload(u8),

        /// Load `long` from local variable
        ///
        /// Format: `lload_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.lload_n)
        (0x1e) = lload_0(),

        /// Load `long` from local variable
        ///
        /// Format: `lload_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.lload_n)
        (0x1f) = lload_1(),

        /// Load `long` from local variable
        ///
        /// Format: `lload_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.lload_n)
        (0x20) = lload_2(),

        /// Load `long` from local variable
        ///
        /// Format: `lload_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.lload_n)
        (0x21) = lload_3(),

        /// Multiply `long`
        ///
        /// Format: `lmul`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.lmul)
        (0x69) = lmul(),

        /// Negate `long`
        ///
        /// Format: `lneg`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.lneg)
        (0x75) = lneg(),

        /// Boolean OR `long`
        ///
        /// Format: `lor`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.lor)
        (0x81) = lor(),

        /// Remainder `long`
        ///
        /// Format: `lrem`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.lrem)
        (0x71) = lrem(),

        /// Return `long` from method
        ///
        /// Format: `lreturn`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.lreturn)
        (0xad) = lreturn(),

        /// Shift left `long`
        ///
        /// Format: `lshl`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.lshl)
        (0x79) = lshl(),

        /// Arithmetic shift right `long`
        ///
        /// Format: `lshr`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.lshr)
        (0x7b) = lshr(),

        /// Store `long` into local variable
        ///
        /// Format: `lstore`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.lstore)
        (0x37) = lstore(u8),

        /// Store `long` into local variable
        ///
        /// Format: `lstore_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.lstore_n)
        (0x3f) = lstore_0(),

        /// Store `long` into local variable
        ///
        /// Format: `lstore_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.lstore_n)
        (0x40) = lstore_1(),

        /// Store `long` into local variable
        ///
        /// Format: `lstore_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.lstore_n)
        (0x41) = lstore_2(),

        /// Store `long` into local variable
        ///
        /// Format: `lstore_<n>`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.lstore_n)
        (0x42) = lstore_3(),

        /// Subtract `long`
        ///
        /// Format: `lsub`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.lsub)
        (0x65) = lsub(),

        /// Logical shift right `long`
        ///
        /// Format: `lushr`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.lushr)
        (0x7d) = lushr(),

        /// Boolean XOR `long`
        ///
        /// Format: `lxor`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.lxor)
        (0x83) = lxor(),

        /// Enter monitor for object
        ///
        /// Format: `monitorenter`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.monitorenter)
        (0xc2) = monitorenter(),

        /// Exit monitor for object
        ///
        /// Format: `monitorexit`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.monitorexit)
        (0xc3) = monitorexit(),

        /// Create new multidimensional array
        ///
        /// Format: `multianewarray indexbyte1 indexbyte2 dimensions`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.multianewarray)
        (0xc5) = multianewarray(u16, u8),

        /// Create new object
        ///
        /// Format: `new indexbyte1 indexbyte2`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.new)
        (0xbb) = new(u16),

        /// Create new array
        ///
        /// Format: `newarray atype`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.newarray)
        (0xbc) = newarray(ArrayTypeCode),

        /// Do nothing
        ///
        /// Format: `nop`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.nop)
        (0x0) = nop(),

        /// Pop the top operand stack value
        ///
        /// Format: `pop`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.pop)
        (0x57) = pop(),

        /// Pop the top one or two operand stack values
        ///
        /// Format: `pop2`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.pop2)
        (0x58) = pop2(),

        /// Set field in object
        ///
        /// Format: `putfield indexbyte1 indexbyte2`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.putfield)
        (0xb5) = putfield(u16),

        /// Set static field in class
        ///
        /// Format: `putstatic indexbyte1 indexbyte2`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.putstatic)
        (0xb3) = putstatic(u16),

        /// Return from subroutine
        ///
        /// Format: `ret index`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.ret)
        (0xa9) = ret(u8),

        /// Return `void` from method
        ///
        /// Format: `return`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.return)
        (0xb1) = r#return(),

        /// Load `short` from array
        ///
        /// Format: `saload`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.saload)
        (0x35) = saload(),

        /// Store into `short` array
        ///
        /// Format: `sastore`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.sastore)
        (0x56) = sastore(),

        /// Push `short`
        ///
        /// Format: `sipush byte1 byte2`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.sipush)
        (0x11) = sipush(u16),

        /// Swap the top two operand stack values
        ///
        /// Format: `swap`
        ///
        /// Details: [Java SE 8 Specification](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.swap)
        (0x5f) = swap()

    }
}
