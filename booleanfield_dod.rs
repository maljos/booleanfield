use std::fmt;
use std::marker::PhantomData;

// Import display components from the new module
use super::booleanfield_display::BooleanDisplayConfig;

// --- Bit-Packed Data Component ---

/// Represents the three states of a boolean field to simplify logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OptionBool {
    True,
    False,
    Null,
}

impl From<Option<bool>> for OptionBool {
    fn from(opt: Option<bool>) -> Self {
        match opt {
            Some(true) => OptionBool::True,
            Some(false) => OptionBool::False,
            None => OptionBool::Null,
        }
    }
}

impl From<OptionBool> for Option<bool> {
    fn from(ob: OptionBool) -> Self {
        match ob {
            OptionBool::True => Some(true),
            OptionBool::False => Some(false),
            OptionBool::Null => None,
        }
    }
}

/// Encodes the state (not_null, default, value) into a single u8 based on the 13 valid states.
/// Returns an error for any invalid combination.
fn encode_state(not_null: bool, default: OptionBool, value: OptionBool) -> Result<u8, &'static str> {
    use OptionBool::*;
    match (not_null, default, value) {
        // N=F
        (false, False, False) => Ok(0),
        (false, False, True) => Ok(1),
        (false, False, Null) => Ok(2),
        (false, True, False) => Ok(4),
        (false, True, True) => Ok(5),
        (false, True, Null) => Ok(6),
        (false, Null, False) => Ok(8),
        (false, Null, True) => Ok(9),
        (false, Null, Null) => Ok(10),
        // N=T
        (true, False, False) => Ok(16),
        (true, False, True) => Ok(17),
        (true, True, False) => Ok(20),
        (true, True, True) => Ok(21),
        // Invalid combinations are not listed and will result in an error.
        _ => Err("Invalid state combination"),
    }
}

/// Decodes a u8 state into its components (not_null, default, value).
fn decode_state(state: u8) -> Result<(bool, OptionBool, OptionBool), &'static str> {
    use OptionBool::*;
    match state {
        0 => Ok((false, False, False)),
        1 => Ok((false, False, True)),
        2 => Ok((false, False, Null)),
        4 => Ok((false, True, False)),
        5 => Ok((false, True, True)),
        6 => Ok((false, True, Null)),
        8 => Ok((false, Null, False)),
        9 => Ok((false, Null, True)),
        10 => Ok((false, Null, Null)),
        16 => Ok((true, False, False)),
        17 => Ok((true, False, True)),
        20 => Ok((true, True, False)),
        21 => Ok((true, True, True)),
        _ => Err("Invalid packed state"),
    }
}

/// A memory-optimized boolean data structure using a single byte.
/// It stores the value, default, and not_null constraint in one u8.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct PackedBooleanData(u8);

impl Default for PackedBooleanData {
    /// The default state is N=F, D=N, V=N (state 10), which matches the old `new()` behavior.
    fn default() -> Self {
        Self(10)
    }
}

impl PackedBooleanData {
    /// Decodes the byte to get the full state.
    /// Panics if the internal state is invalid, which should never happen with correct logic.
    fn get_full_state(&self) -> (bool, OptionBool, OptionBool) {
        decode_state(self.0).expect("Internal state of PackedBooleanData is invalid")
    }

    pub fn value(&self) -> Option<bool> {
        self.get_full_state().2.into()
    }

    pub fn default_value(&self) -> Option<bool> {
        self.get_full_state().1.into()
    }

    pub fn not_null(&self) -> bool {
        self.get_full_state().0
    }
}

/// Operations that can be performed on BooleanData
pub(crate) struct BooleanOps;

impl BooleanOps {
    /// Creates a new BooleanData with default values
    pub fn new_data() -> PackedBooleanData {
        PackedBooleanData::default()
    }

    /// Sets the NOT NULL constraint
    pub fn set_not_null(data: &mut PackedBooleanData) {
        let (_, mut default, mut value) = data.get_full_state();

        // If default is NULL, it must become non-NULL (e.g., false).
        if default == OptionBool::Null {
            default = OptionBool::False;
        }

        // If value is NULL, it must become non-NULL, taking the default.
        if value == OptionBool::Null {
            value = default;
        }

        // This encoding must succeed because we've eliminated NULLs.
        data.0 = encode_state(true, default, value).unwrap();
    }

    /// Sets a default value
    pub fn set_default(data: &mut PackedBooleanData, new_default: bool) {
        let (not_null, _, mut value) = data.get_full_state();
        let new_default_ob: OptionBool = Some(new_default).into();

        // If the current value is NULL, it takes on the new default.
        if value == OptionBool::Null {
            value = new_default_ob;
        }

        // This encoding must succeed as the default is not NULL.
        data.0 = encode_state(not_null, new_default_ob, value).unwrap();
    }

    /// Sets a new value with validation
    pub fn set_value(data: &mut PackedBooleanData, value: Option<bool>) -> Result<(), String> {
        let (not_null, default, _) = data.get_full_state();
        let new_value_ob: OptionBool = value.into();

        // Enforce the NOT NULL contract.
        if not_null && new_value_ob == OptionBool::Null {
            return Err("Field cannot be NULL".to_string());
        }

        // The encode function will also catch invalid states, but this check is more user-friendly.
        match encode_state(not_null, default, new_value_ob) {
            Ok(new_state) => {
                data.0 = new_state;
                Ok(())
            }
            Err(e) => Err(e.to_string()),
        }
    }

    /// Logical AND operation with three-state logic
    pub fn and(a: &PackedBooleanData, b: &PackedBooleanData) -> PackedBooleanData {
        let value = match (a.value(), b.value()) {
            (Some(true), Some(true)) => Some(true),
            (_, Some(false)) | (Some(false), _) => Some(false),
            _ => None,
        };
        // Result inherits constraints from 'a'.
        let (not_null, default, _) = a.get_full_state();
        let new_state = encode_state(not_null, default, value.into()).unwrap();
        PackedBooleanData(new_state)
    }

    /// Logical OR operation with three-state logic
    pub fn or(a: &PackedBooleanData, b: &PackedBooleanData) -> PackedBooleanData {
        let value = match (a.value(), b.value()) {
            (Some(true), _) | (_, Some(true)) => Some(true),
            (Some(false), Some(false)) => Some(false),
            _ => None,
        };
        // Result inherits constraints from 'a'.
        let (not_null, default, _) = a.get_full_state();
        let new_state = encode_state(not_null, default, value.into()).unwrap();
        PackedBooleanData(new_state)
    }

    /// Logical NOT operation with three-state logic
    pub fn not(data: &PackedBooleanData) -> PackedBooleanData {
        let value = data.value().map(|val| !val);
        // Result inherits constraints.
        let (not_null, default, _) = data.get_full_state();
        let new_state = encode_state(not_null, default, value.into()).unwrap();
        PackedBooleanData(new_state)
    }

    /// Returns the SQL type definition as a string with all constraints
    pub fn to_sql(data: &PackedBooleanData) -> String {
        let mut sql = "BOOLEAN".to_string();

        if data.not_null() {
            sql.push_str(" NOT NULL");
        }

        if let Some(default_val) = data.default_value() {
            sql.push_str(" DEFAULT ");
            if default_val {
                sql.push_str("TRUE");
            } else {
                sql.push_str("FALSE");
            }
        }
        sql
    }
}

// --- Convenience Wrapper ---

/// A read-only view of the boolean data that provides safe access to the packed boolean state.
///
/// This type ensures that the internal `PackedBooleanData` remains immutable while
/// exposing only the necessary operations to query the boolean state.
#[derive(Debug, Clone, Copy)]
pub struct BooleanDataView<'a>(&'a PackedBooleanData);

impl<'a> BooleanDataView<'a> {
    /// Get the boolean value if set
    pub fn get_value(&self) -> Option<bool> {
        self.0.value()
    }

    /// Check if the field is not null
    pub fn is_not_null(&self) -> bool {
        self.0.not_null()
    }
    
    /// Get the default value if set
    pub fn default_value(&self) -> Option<bool> {
        self.0.default_value()
    }
}

/// A boolean field that combines storage optimization with display configuration.
///
/// This type provides a high-level interface for working with boolean values
/// while internally using a packed representation for memory efficiency.
///
/// # Examples
///
/// ```
/// use dbform::libs::libs_fieldtype::booleanfield_dod::BooleanField;
///
/// // Create a field with NOT NULL constraint and default value
/// let field = BooleanField::<&str>::new()
///     .not_null()
///     .default(true);
///
/// // When NOT NULL is set, the value is automatically set to the default
/// assert_eq!(field.get_value(), Some(false));
/// 
/// // After setting a value, it will be used
/// let mut field = field;
/// field.set_value(Some(true)).unwrap();
/// assert_eq!(field.get_value(), Some(true));
/// ```
#[derive(Clone)]
pub struct BooleanField<T: fmt::Display + Clone + 'static> {
    /// The underlying packed boolean data
    data: PackedBooleanData,
    /// Configuration for display formatting
    display_config: Option<BooleanDisplayConfig<T>>,
    _marker: PhantomData<T>,
}

impl<T: fmt::Display + Clone + 'static> BooleanField<T> {
    pub fn new() -> Self {
        Self {
            data: BooleanOps::new_data(),
            display_config: None,
            _marker: PhantomData,
        }
    }

    /// Get read-only access to the underlying boolean data
    pub fn data(&self) -> BooleanDataView<'_> {
        BooleanDataView(&self.data)
    }

    /// Get a mutable reference to the display configuration
    pub fn display_config_mut(&mut self) -> &mut Option<BooleanDisplayConfig<T>> {
        &mut self.display_config
    }

    /// Get a reference to the display configuration
    pub fn display_config(&self) -> Option<&BooleanDisplayConfig<T>> {
        self.display_config.as_ref()
    }

    pub fn with_display(
        mut self,
        false_display: T,
        true_display: T,
        null_display: Option<T>,
    ) -> Self {
        self.display_config = Some(BooleanDisplayConfig {
            false_display,
            true_display,
            null_display,
        });
        self
    }

    /// Sets the NOT NULL constraint on the boolean field.
    ///
    /// # Returns
    /// Returns `Self` to allow method chaining.
    ///
    /// # Example
    /// ```
    /// use dbform::libs::libs_fieldtype::booleanfield_dod::BooleanField;
    ///
    /// let field = BooleanField::<&str>::new().not_null();
    /// assert!(field.is_not_null());
    /// ```
    pub fn not_null(mut self) -> Self {
        BooleanOps::set_not_null(&mut self.data);
        self
    }

    /// Gets the current value of the boolean field.
    ///
    /// # Returns
    /// - `Some(true)` if the value is set to true
    /// - `Some(false)` if the value is set to false
    /// - `None` if the value is not set (NULL)
    pub fn get_value(&self) -> Option<bool> {
        self.data().get_value()
    }

    /// Checks if the field has the NOT NULL constraint set.
    ///
    /// # Returns
    /// - `true` if the field cannot be NULL
    /// - `false` if the field can be NULL
    pub fn is_not_null(&self) -> bool {
        self.data().is_not_null()
    }

    /// Sets a default value for the boolean field.
    ///
    /// # Arguments
    /// * `default` - The default boolean value to set
    ///
    /// # Returns
    /// Returns `Self` to allow method chaining.
    ///
    /// # Example
    /// ```
    /// use dbform::libs::libs_fieldtype::booleanfield_dod::BooleanField;
    ///
    /// let field = BooleanField::<&str>::new();
    /// assert_eq!(field.get_value(), None); // Default value is None (NULL)
    /// ```
    pub fn default(mut self, default: bool) -> Self {
        BooleanOps::set_default(&mut self.data, default);
        self
    }

    /// Sets a new value for the boolean field with validation.
    ///
    /// # Arguments
    /// * `value` - The value to set (`Some(true)`, `Some(false)`, or `None` for NULL)
    ///
    /// # Returns
    /// - `Ok(())` if the value was set successfully
    /// - `Err(String)` if the value violates any constraints (e.g., setting NULL on a NOT NULL field)
    ///
    /// # Example
    /// ```
    /// use dbform::libs::libs_fieldtype::booleanfield_dod::BooleanField;
    ///
    /// let mut field = BooleanField::<&str>::new();
    /// assert!(field.set_value(Some(true)).is_ok());
    /// assert_eq!(field.get_value(), Some(true));
    /// ```
    pub fn set_value(&mut self, value: Option<bool>) -> Result<(), String> {
        BooleanOps::set_value(&mut self.data, value)
    }

    /// Performs a logical AND operation with another boolean field.
    ///
    /// # Arguments
    /// * `other` - The other boolean field to AND with
    ///
    /// # Returns
    /// A new `BooleanField` that is the result of the AND operation.
    ///
    /// # Truth Table
    /// 
    /// | A     | B     | A AND B |
    /// |-------|-------|---------|
    /// | true  | true  | true    |
    /// | true  | false | false   |
    /// | true  | NULL  | NULL    |
    /// | false | true  | false   |
    /// | false | false | false   |
    /// | false | NULL  | false   |
    /// | NULL  | true  | NULL    |
    /// | NULL  | false | false   |
    /// | NULL  | NULL  | NULL    |
    pub fn and(self, other: Self) -> Self {
        Self {
            data: BooleanOps::and(&self.data, &other.data),
            display_config: self.display_config,
            _marker: PhantomData,
        }
    }

    /// Performs a logical OR operation with another boolean field.
    ///
    /// # Arguments
    /// * `other` - The other boolean field to OR with
    ///
    /// # Returns
    /// A new `BooleanField` that is the result of the OR operation.
    ///
    /// # Truth Table
    /// 
    /// | A     | B     | A OR B  |
    /// |-------|-------|---------|
    /// | true  | true  | true    |
    /// | true  | false | true    |
    /// | true  | NULL  | true    |
    /// | false | true  | true    |
    /// | false | false | false   |
    /// | false | NULL  | NULL    |
    /// | NULL  | true  | true    |
    /// | NULL  | false | NULL    |
    /// | NULL  | NULL  | NULL    |
    ///
    /// # Example
    /// ```
    /// use dbform::libs::libs_fieldtype::booleanfield_dod::BooleanField;
    ///
    /// let mut true_field = BooleanField::<&str>::new();
    /// true_field.set_value(Some(true)).unwrap();
    /// let mut false_field = BooleanField::<&str>::new();
    /// false_field.set_value(Some(false)).unwrap();
    /// let result = true_field.or(false_field);
    /// assert_eq!(result.get_value(), Some(true));
    /// ```
    pub fn or(self, other: Self) -> Self {
        Self {
            data: BooleanOps::or(&self.data, &other.data),
            display_config: self.display_config,
            _marker: PhantomData,
        }
    }

    /// Performs a logical NOT operation on the boolean field.
    ///
    /// # Returns
    /// A new `BooleanField` that is the logical negation of this field.
    ///
    /// # Truth Table
    /// 
    /// | A     | NOT A |
    /// |-------|-------|
    /// | true  | false |
    /// | false | true  |
    /// | NULL  | NULL  |
    ///
    /// # Example
    /// ```
    /// use dbform::libs::libs_fieldtype::booleanfield_dod::BooleanField;
    ///
    /// let mut true_field = BooleanField::<&str>::new();
    /// true_field.set_value(Some(true)).unwrap();
    /// let not_field = true_field.not();
    /// assert_eq!(not_field.get_value(), Some(false));
    /// ```
    pub fn not(self) -> Self {
        Self {
            data: BooleanOps::not(&self.data),
            display_config: self.display_config,
            _marker: PhantomData,
        }
    }

    /// Returns a display string representation of the boolean field.
    ///
    /// If a custom display configuration has been set using `with_display_config`,
    /// it will be used to format the output. Otherwise, it falls back to the
    /// default string representations ("true", "false", "NULL").
    ///
    /// # Returns
    /// A `String` representing the boolean value according to the display configuration.
    ///
    /// # Example
    /// ```
    /// use dbform::libs::libs_fieldtype::booleanfield_dod::BooleanField;
    ///
    /// let mut field = BooleanField::<&str>::new();
    /// field.set_value(Some(true)).unwrap();
    /// assert_eq!(field.display(), "true");
    ///
    /// let mut field = BooleanField::<&str>::new();
    /// field.set_value(Some(false)).unwrap();
    /// assert_eq!(field.display(), "false");
    ///
    /// let field = BooleanField::<&str>::new();
    /// assert_eq!(field.display(), "NULL");
    /// ```
    /// Returns a display string representation of the boolean field.
    ///
    /// If a custom display configuration has been set using `with_display_config`,
    /// it will be used to format the output. Otherwise, it falls back to the
    /// default string representations ("true", "false", "NULL").
    ///
    /// # Returns
    /// A `String` representing the boolean value according to the display configuration.
    ///
    /// # Example
    /// ```
    /// use dbform::libs::libs_fieldtype::booleanfield_dod::BooleanField;
    ///
    /// let mut field = BooleanField::<&str>::new();
    /// field.set_value(Some(true)).unwrap();
    /// assert_eq!(field.display(), "true");
    ///
    /// let mut field = BooleanField::<&str>::new();
    /// field.set_value(Some(false)).unwrap();
    /// assert_eq!(field.display(), "false");
    ///
    /// let field = BooleanField::<&str>::new();
    /// assert_eq!(field.display(), "NULL");
    /// ```
    pub fn display(&self) -> String {
        match self.display_config.as_ref() {
            Some(config) => {
                let value = self.data.value();
                match value {
                    Some(true) => config.true_display.to_string(),
                    Some(false) => config.false_display.to_string(),
                    None => config.null_display.as_ref().map_or_else(
                        || "NULL".to_string(),
                        |s| s.to_string()
                    ),
                }
            }
            None => match self.data.value() {
                Some(true) => "true".to_string(),
                Some(false) => "false".to_string(),
                None => "NULL".to_string(),
            }
        }
    }

    /// Returns the SQL type definition as a string with all constraints
    pub fn to_sql(&self) -> String {
        BooleanOps::to_sql(&self.data)
    }
}

impl<T: fmt::Display + Clone + 'static> Default for BooleanField<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: fmt::Display + Clone + 'static> fmt::Display for BooleanField<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display())
    }
}

impl<T: fmt::Display + Clone + 'static> fmt::Debug for BooleanField<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BooleanField")
            .field("value", &self.data.value())
            .field("not_null", &self.data.not_null())
            .field("default", &self.data.default_value())
            .finish()
    }
}

impl<T: fmt::Display + Clone + 'static> From<bool> for BooleanField<T> {
    fn from(value: bool) -> Self {
        let mut field = Self::new();
        // This should not fail
        field.set_value(Some(value)).unwrap();
        field
    }
}

impl<T: fmt::Display + Clone + 'static> From<Option<bool>> for BooleanField<T> {
    fn from(value: Option<bool>) -> Self {
        let mut field = Self::new();
        // This should not fail as not_null is false by default
        field.set_value(value).unwrap();
        field
    }
}
