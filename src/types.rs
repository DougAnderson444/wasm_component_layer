use anyhow::*;
use crate::values::*;
use std::mem::*;
use std::ops::*;
use std::sync::*;
use wasmtime_environ::component::*;
use wasmtime_environ::*;

/// An owned and `'static` handle for type information in a component.
///
/// The components here are:
///
/// * `index` - a `TypeFooIndex` defined in the `wasmtime_environ` crate. This
///   then points into the next field of...
///
/// * `types` - this is an allocation originally created from compilation and is
///   stored in a compiled `Component`. This contains all types necessary and
///   information about recursive structures and all other type information
///   within the component. The above `index` points into this structure.
#[derive(Clone)]
struct Handle<T> {
    index: T,
    instance: Arc<crate::InstanceInner>
}

impl<T> Handle<T> {
    fn new(index: T, instance: Arc<crate::InstanceInner>) -> Handle<T> {
        Handle {
            index,
            instance
        }
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Handle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Handle")
            .field("index", &self.index)
            .finish()
    }
}

impl<T: PartialEq> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        // FIXME: This is an overly-restrictive definition of equality in that it doesn't consider types to be
        // equal unless they refer to the same declaration in the same component.  It's a good shortcut for the
        // common case, but we should also do a recursive structural equality test if the shortcut test fails.
        self.index == other.index && Arc::ptr_eq(&self.instance, &other.instance)
    }
}

impl<T: Eq> Eq for Handle<T> {}

/// A `list` interface type
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct List(Handle<TypeListIndex>);

impl List {
    /// Instantiate this type with the specified `values`.
    pub fn new_val(&self, values: Box<[Value]>) -> Result<Value> {
        Ok(Value::List(crate::values::List::new(self, values)?))
    }

    pub(crate) fn from(index: TypeListIndex, instance: Arc<crate::InstanceInner>) -> Self {
        List(Handle::new(index, instance))
    }

    /// Retreive the element type of this `list`.
    pub fn ty(&self) -> Type {
        Type::from(&self.0.instance.component.types[self.0.index].element, self.0.instance.clone())
    }
}

/// A field declaration belonging to a `record`
pub struct Field<'a> {
    /// The name of the field
    pub name: &'a str,
    /// The type of the field
    pub ty: Type,
}

/// A `record` interface type
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Record(Handle<TypeRecordIndex>);

impl Record {
    /// Instantiate this type with the specified `values`.
    pub fn new_val<'a>(&self, values: impl IntoIterator<Item = (&'a str, Value)>) -> Result<Value> {
        Ok(Value::Record(crate::values::Record::new(self, values)?))
    }

    pub(crate) fn from(index: TypeRecordIndex, ty: Arc<crate::InstanceInner>) -> Self {
        Record(Handle::new(index, ty))
    }

    /// Retrieve the fields of this `record` in declaration order.
    pub fn fields(&self) -> impl ExactSizeIterator<Item = Field<'_>> {
        self.0.instance.component.types[self.0.index].fields.iter().map(|field| Field {
            name: &field.name,
            ty: Type::from(&field.ty, self.0.instance.clone()),
        })
    }
}

/// A `tuple` interface type
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Tuple(Handle<TypeTupleIndex>);

impl Tuple {
    /// Instantiate this type ith the specified `values`.
    pub fn new_val(&self, values: Box<[Value]>) -> Result<Value> {
        Ok(Value::Tuple(crate::values::Tuple::new(self, values)?))
    }

    pub(crate) fn from(index: TypeTupleIndex, ty: Arc<crate::InstanceInner>) -> Self {
        Tuple(Handle::new(index, ty))
    }

    /// Retrieve the types of the fields of this `tuple` in declaration order.
    pub fn types(&self) -> impl ExactSizeIterator<Item = Type> + '_ {
        self.0.instance.component.types[self.0.index]
            .types
            .iter()
            .map(|ty| Type::from(ty, self.0.instance.clone()))
    }
}

/// A case declaration belonging to a `variant`
pub struct Case<'a> {
    /// The name of the case
    pub name: &'a str,
    /// The optional payload type of the case
    pub ty: Option<Type>,
}

/// A `variant` interface type
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Variant(Handle<TypeVariantIndex>);

impl Variant {
    /// Instantiate this type with the specified case `name` and `value`.
    pub fn new_val(&self, name: &str, value: Option<Value>) -> Result<Value> {
        Ok(Value::Variant(crate::values::Variant::new(self, name, value)?))
    }

    pub(crate) fn from(index: TypeVariantIndex, ty: Arc<crate::InstanceInner>) -> Self {
        Variant(Handle::new(index, ty))
    }

    /// Retrieve the cases of this `variant` in declaration order.
    pub fn cases(&self) -> impl ExactSizeIterator<Item = Case> {
        self.0.instance.component.types[self.0.index].cases.iter().map(|case| Case {
            name: &case.name,
            ty: case
                .ty
                .as_ref()
                .map(|ty| Type::from(ty, self.0.instance.clone())),
        })
    }
}

/// An `enum` interface type
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Enum(Handle<TypeEnumIndex>);

impl Enum {
    /// Instantiate this type with the specified case `name`.
    pub fn new_val(&self, name: &str) -> Result<Value> {
        Ok(Value::Enum(crate::values::Enum::new(self, name)?))
    }

    pub(crate) fn from(index: TypeEnumIndex, ty: Arc<crate::InstanceInner>) -> Self {
        Enum(Handle::new(index, ty))
    }

    /// Retrieve the names of the cases of this `enum` in declaration order.
    pub fn names(&self) -> impl ExactSizeIterator<Item = &str> {
        self.0.instance.component.types[self.0.index]
            .names
            .iter()
            .map(|name| name.deref())
    }
}

/// A `union` interface type
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Union(Handle<TypeUnionIndex>);

impl Union {
    /// Instantiate this type with the specified `discriminant` and `value`.
    pub fn new_val(&self, discriminant: u32, value: Value) -> Result<Value> {
        Ok(Value::Union(crate::values::Union::new(self, discriminant, value)?))
    }

    pub(crate) fn from(index: TypeUnionIndex, ty: Arc<crate::InstanceInner>) -> Self {
        Union(Handle::new(index, ty))
    }

    /// Retrieve the types of the cases of this `union` in declaration order.
    pub fn types(&self) -> impl ExactSizeIterator<Item = Type> + '_ {
        self.0.instance.component.types[self.0.index]
            .types
            .iter()
            .map(|ty| Type::from(ty, self.0.instance.clone()))
    }
}

/// An `option` interface type
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct OptionType(Handle<TypeOptionIndex>);

impl OptionType {
    /// Instantiate this type with the specified `value`.
    pub fn new_val(&self, value: Option<Value>) -> Result<Value> {
        Ok(Value::Option(crate::values::OptionVal::new(self, value)?))
    }

    pub(crate) fn from(index: TypeOptionIndex, ty: Arc<crate::InstanceInner>) -> Self {
        OptionType(Handle::new(index, ty))
    }

    /// Retrieve the type parameter for this `option`.
    pub fn ty(&self) -> Type {
        Type::from(&self.0.instance.component.types[self.0.index].ty, self.0.instance.clone())
    }
}

/// An `expected` interface type
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ResultType(Handle<TypeResultIndex>);

impl ResultType {
    /// Instantiate this type with the specified `value`.
    pub fn new_val(&self, value: Result<Option<Value>, Option<Value>>) -> Result<Value> {
        Ok(Value::Result(crate::values::ResultVal::new(self, value)?))
    }

    pub(crate) fn from(index: TypeResultIndex, ty: Arc<crate::InstanceInner>) -> Self {
        ResultType(Handle::new(index, ty))
    }

    /// Retrieve the `ok` type parameter for this `option`.
    pub fn ok(&self) -> Option<Type> {
        Some(Type::from(
            self.0.instance.component.types[self.0.index].ok.as_ref()?,
            self.0.instance.clone(),
        ))
    }

    /// Retrieve the `err` type parameter for this `option`.
    pub fn err(&self) -> Option<Type> {
        Some(Type::from(
            self.0.instance.component.types[self.0.index].err.as_ref()?,
            self.0.instance.clone(),
        ))
    }
}

/// A `flags` interface type
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Flags(Handle<TypeFlagsIndex>);

impl Flags {
    /// Instantiate this type with the specified flag `names`.
    pub fn new_val(&self, names: &[&str]) -> Result<Value> {
        Ok(Value::Flags(crate::values::Flags::new(self, names)?))
    }

    pub(crate) fn from(index: TypeFlagsIndex, ty: Arc<crate::InstanceInner>) -> Self {
        Flags(Handle::new(index, ty))
    }

    /// Retrieve the names of the flags of this `flags` type in declaration order.
    pub fn names(&self) -> impl ExactSizeIterator<Item = &str> {
        self.0.instance.component.types[self.0.index]
            .names
            .iter()
            .map(|name| name.deref())
    }

    pub(crate) fn canonical_abi(&self) -> &CanonicalAbiInfo {
        &self.0.instance.component.types[self.0.index].abi
    }
}

/// Represents a component model interface type
#[derive(Clone, PartialEq, Eq, Debug)]
#[allow(missing_docs)]
pub enum Type {
    Bool,
    S8,
    U8,
    S16,
    U16,
    S32,
    U32,
    S64,
    U64,
    Float32,
    Float64,
    Char,
    String,
    List(List),
    Record(Record),
    Tuple(Tuple),
    Variant(Variant),
    Enum(Enum),
    Union(Union),
    Option(OptionType),
    Result(ResultType),
    Flags(Flags),
    Own(()),
    Borrow(()),
}

impl Type {
    /// Retrieve the inner [`List`] of a [`Type::List`].
    ///
    /// # Panics
    ///
    /// This will panic if `self` is not a [`Type::List`].
    pub fn unwrap_list(&self) -> &List {
        if let Type::List(handle) = self {
            &handle
        } else {
            panic!("attempted to unwrap a {} as a list", self.desc())
        }
    }

    /// Retrieve the inner [`Record`] of a [`Type::Record`].
    ///
    /// # Panics
    ///
    /// This will panic if `self` is not a [`Type::Record`].
    pub fn unwrap_record(&self) -> &Record {
        if let Type::Record(handle) = self {
            &handle
        } else {
            panic!("attempted to unwrap a {} as a record", self.desc())
        }
    }

    /// Retrieve the inner [`Tuple`] of a [`Type::Tuple`].
    ///
    /// # Panics
    ///
    /// This will panic if `self` is not a [`Type::Tuple`].
    pub fn unwrap_tuple(&self) -> &Tuple {
        if let Type::Tuple(handle) = self {
            &handle
        } else {
            panic!("attempted to unwrap a {} as a tuple", self.desc())
        }
    }

    /// Retrieve the inner [`Variant`] of a [`Type::Variant`].
    ///
    /// # Panics
    ///
    /// This will panic if `self` is not a [`Type::Variant`].
    pub fn unwrap_variant(&self) -> &Variant {
        if let Type::Variant(handle) = self {
            &handle
        } else {
            panic!("attempted to unwrap a {} as a variant", self.desc())
        }
    }

    /// Retrieve the inner [`Enum`] of a [`Type::Enum`].
    ///
    /// # Panics
    ///
    /// This will panic if `self` is not a [`Type::Enum`].
    pub fn unwrap_enum(&self) -> &Enum {
        if let Type::Enum(handle) = self {
            &handle
        } else {
            panic!("attempted to unwrap a {} as a enum", self.desc())
        }
    }

    /// Retrieve the inner [`Union`] of a [`Type::Union`].
    ///
    /// # Panics
    ///
    /// This will panic if `self` is not a [`Type::Union`].
    pub fn unwrap_union(&self) -> &Union {
        if let Type::Union(handle) = self {
            &handle
        } else {
            panic!("attempted to unwrap a {} as a union", self.desc())
        }
    }

    /// Retrieve the inner [`OptionType`] of a [`Type::Option`].
    ///
    /// # Panics
    ///
    /// This will panic if `self` is not a [`Type::Option`].
    pub fn unwrap_option(&self) -> &OptionType {
        if let Type::Option(handle) = self {
            &handle
        } else {
            panic!("attempted to unwrap a {} as a option", self.desc())
        }
    }

    /// Retrieve the inner [`ResultType`] of a [`Type::Result`].
    ///
    /// # Panics
    ///
    /// This will panic if `self` is not a [`Type::Result`].
    pub fn unwrap_result(&self) -> &ResultType {
        if let Type::Result(handle) = self {
            &handle
        } else {
            panic!("attempted to unwrap a {} as a result", self.desc())
        }
    }

    /// Retrieve the inner [`Flags`] of a [`Type::Flags`].
    ///
    /// # Panics
    ///
    /// This will panic if `self` is not a [`Type::Flags`].
    pub fn unwrap_flags(&self) -> &Flags {
        if let Type::Flags(handle) = self {
            &handle
        } else {
            panic!("attempted to unwrap a {} as a flags", self.desc())
        }
    }

    /// Retrieve the inner [`ResourceType`] of a [`Type::Own`].
    ///
    /// # Panics
    ///
    /// This will panic if `self` is not a [`Type::Own`].
    pub fn unwrap_own(&self) -> &() {
        match self {
            Type::Own(ty) => ty,
            _ => panic!("attempted to unwrap a {} as a own", self.desc()),
        }
    }

    /// Retrieve the inner [`ResourceType`] of a [`Type::Borrow`].
    ///
    /// # Panics
    ///
    /// This will panic if `self` is not a [`Type::Borrow`].
    pub fn unwrap_borrow(&self) -> &() {
        match self {
            Type::Borrow(ty) => ty,
            _ => panic!("attempted to unwrap a {} as a own", self.desc()),
        }
    }

    pub(crate) fn check(&self, value: &Value) -> Result<()> {
        let other = &value.ty();
        if self == other {
            Ok(())
        } else if discriminant(self) != discriminant(other) {
            Err(anyhow!(
                "type mismatch: expected {}, got {}",
                self.desc(),
                other.desc()
            ))
        } else {
            Err(anyhow!(
                "type mismatch for {}, possibly due to mixing distinct composite types",
                self.desc()
            ))
        }
    }

    /// Convert the specified `InterfaceType` to a `Type`.
    pub(crate) fn from(ty: &InterfaceType, instance: Arc<crate::InstanceInner>) -> Self {
        match ty {
            InterfaceType::Bool => Type::Bool,
            InterfaceType::S8 => Type::S8,
            InterfaceType::U8 => Type::U8,
            InterfaceType::S16 => Type::S16,
            InterfaceType::U16 => Type::U16,
            InterfaceType::S32 => Type::S32,
            InterfaceType::U32 => Type::U32,
            InterfaceType::S64 => Type::S64,
            InterfaceType::U64 => Type::U64,
            InterfaceType::Float32 => Type::Float32,
            InterfaceType::Float64 => Type::Float64,
            InterfaceType::Char => Type::Char,
            InterfaceType::String => Type::String,
            InterfaceType::List(index) => Type::List(List::from(*index, instance)),
            InterfaceType::Record(index) => Type::Record(Record::from(*index, instance)),
            InterfaceType::Tuple(index) => Type::Tuple(Tuple::from(*index, instance)),
            InterfaceType::Variant(index) => Type::Variant(Variant::from(*index, instance)),
            InterfaceType::Enum(index) => Type::Enum(Enum::from(*index, instance)),
            InterfaceType::Union(index) => Type::Union(Union::from(*index, instance)),
            InterfaceType::Option(index) => Type::Option(OptionType::from(*index, instance)),
            InterfaceType::Result(index) => Type::Result(ResultType::from(*index, instance)),
            InterfaceType::Flags(index) => Type::Flags(Flags::from(*index, instance)),
            InterfaceType::Own(index) => Type::Bool, // todo Type::Own(instance.resource_type(*index)),
            InterfaceType::Borrow(index) => Type::Bool // todo Type::Borrow(instance.resource_type(*index)),
        }
    }

    fn desc(&self) -> &'static str {
        match self {
            Type::Bool => "bool",
            Type::S8 => "s8",
            Type::U8 => "u8",
            Type::S16 => "s16",
            Type::U16 => "u16",
            Type::S32 => "s32",
            Type::U32 => "u32",
            Type::S64 => "s64",
            Type::U64 => "u64",
            Type::Float32 => "float32",
            Type::Float64 => "float64",
            Type::Char => "char",
            Type::String => "string",
            Type::List(_) => "list",
            Type::Record(_) => "record",
            Type::Tuple(_) => "tuple",
            Type::Variant(_) => "variant",
            Type::Enum(_) => "enum",
            Type::Union(_) => "union",
            Type::Option(_) => "option",
            Type::Result(_) => "result",
            Type::Flags(_) => "flags",
            Type::Own(_) => "own",
            Type::Borrow(_) => "borrow",
        }
    }
}