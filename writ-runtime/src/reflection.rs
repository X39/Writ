//! ReflectionIndex: lazy cache for Type heap objects allocated on demand.
//!
//! Maps (module_idx, typedef_idx) to a HeapRef pointing to an allocated Type
//! class object on the GC heap. Objects are created on first access and cached
//! permanently. All cached HeapRefs are registered as GC roots so they survive
//! collection even when no script-side references exist.

use rustc_hash::FxHashMap;
use writ_module::heap::read_string;

use crate::gc::GcHeap;
use crate::heap::HeapObject;
use crate::loader::LoadedModule;
use crate::value::{HeapRef, Value};

/// Lazy cache for reflection metadata heap objects.
///
/// All caches are keyed by (module_idx, index) tuples. Objects are allocated
/// on first access and never freed (registered as permanent GC roots).
pub struct ReflectionIndex {
    /// Cache for Type objects: keyed by (module_idx, typedef_0based_idx).
    /// `module_idx = usize::MAX` is reserved for synthetic primitive types.
    pub(crate) type_cache: FxHashMap<(usize, usize), HeapRef>,
    /// Reverse map: HeapRef -> (module_idx, typedef_idx) for Type objects.
    /// Used by TypeFields/TypeMethods/etc. to recover identity from a Type heap object.
    pub(crate) type_reverse: FxHashMap<HeapRef, (usize, usize)>,
    /// Cache for FieldInfo objects: keyed by (module_idx, field_0based_idx).
    pub(crate) field_cache: FxHashMap<(usize, usize), HeapRef>,
    /// Reverse map: HeapRef -> (module_idx, typedef_idx, field_offset) for FieldInfo objects.
    /// Used by FieldInfo.get(instance) to recover which field to read.
    pub(crate) field_reverse: FxHashMap<HeapRef, (usize, usize, usize)>,
    /// Cache for MethodInfo objects: keyed by (module_idx, method_0based_idx).
    pub(crate) method_cache: FxHashMap<(usize, usize), HeapRef>,
    /// Reverse map: HeapRef -> (module_idx, method_idx) for MethodInfo objects.
    /// Used by MethodInfo.invoke(instance, args) to recover which method to call.
    pub(crate) method_reverse: FxHashMap<HeapRef, (usize, usize)>,
    /// Cache for ParameterInfo objects: keyed by (module_idx, param_0based_idx).
    pub(crate) param_cache: FxHashMap<(usize, usize), HeapRef>,
    /// Cache for AttributeInfo objects: keyed by (module_idx, typedef_idx, attr_ordinal).
    pub(crate) attr_cache: FxHashMap<(usize, usize, usize), HeapRef>,
    /// Cache for ContractInfo objects: keyed by (module_idx, contract_0based_idx).
    pub(crate) contract_info_cache: FxHashMap<(usize, usize), HeapRef>,
    /// Cache for method-scoped AttributeInfo objects: keyed by (module_idx, method_idx, attr_ordinal).
    pub(crate) method_attr_cache: FxHashMap<(usize, usize, usize), HeapRef>,
    /// Cache for field-scoped AttributeInfo objects: keyed by (module_idx, field_idx, attr_ordinal).
    pub(crate) field_attr_cache: FxHashMap<(usize, usize, usize), HeapRef>,
}

impl ReflectionIndex {
    /// Create a new empty ReflectionIndex. All caches start empty (lazy, NOT eager).
    pub fn new() -> Self {
        Self {
            type_cache: FxHashMap::default(),
            type_reverse: FxHashMap::default(),
            field_cache: FxHashMap::default(),
            field_reverse: FxHashMap::default(),
            method_cache: FxHashMap::default(),
            method_reverse: FxHashMap::default(),
            param_cache: FxHashMap::default(),
            attr_cache: FxHashMap::default(),
            contract_info_cache: FxHashMap::default(),
            method_attr_cache: FxHashMap::default(),
            field_attr_cache: FxHashMap::default(),
        }
    }

    /// The type_key used for allocating Type class heap objects.
    ///
    /// Encoded as `(virtual_module_idx << 16) | typedef_idx`. The virtual module
    /// is always at index 0 in the domain.
    const TYPE_TYPE_KEY: u32 = (0u32 << 16) | 9u32;

    /// Number of fields in the Type class: name, namespace, kind, is_generic, type_args.
    const TYPE_FIELD_COUNT: usize = 5;

    /// Get or allocate a Type heap object for the given module TypeDef.
    ///
    /// On first call: allocates a struct on the heap, fills in the 4 Type fields
    /// from module metadata, caches and returns the HeapRef.
    /// On subsequent calls: returns the cached HeapRef directly.
    pub fn get_or_alloc_type(
        &mut self,
        module_idx: usize,
        typedef_idx: usize,
        heap: &mut dyn GcHeap,
        modules: &[LoadedModule],
    ) -> HeapRef {
        let key = (module_idx, typedef_idx);
        if let Some(&href) = self.type_cache.get(&key) {
            return href;
        }

        // Allocate the Type class object
        let href = heap.alloc_struct(Self::TYPE_TYPE_KEY, Self::TYPE_FIELD_COUNT);

        // Fill in fields from module metadata
        let module = &modules[module_idx];
        if typedef_idx < module.module.type_defs.len() {
            let td = &module.module.type_defs[typedef_idx];

            // Field 0: name (string)
            let name = read_string(&module.module.string_heap, td.name)
                .unwrap_or("")
                .to_owned();
            let name_href = heap.alloc_string(&name);
            let _ = heap.set_field(href, 0, Value::Ref(name_href));

            // Field 1: namespace (string)
            let namespace = read_string(&module.module.string_heap, td.namespace)
                .unwrap_or("")
                .to_owned();
            let ns_href = heap.alloc_string(&namespace);
            let _ = heap.set_field(href, 1, Value::Ref(ns_href));

            // Field 2: kind (string) — map TypeDef kind byte to name
            let kind_str = match td.kind {
                0 => "struct",
                1 => "enum",
                2 => "entity",
                3 => "component",
                4 => "class",
                _ => "unknown",
            };
            let kind_href = heap.alloc_string(kind_str);
            let _ = heap.set_field(href, 2, Value::Ref(kind_href));

            // Field 3: is_generic (bool) — scan GenericParam table for any params owned by this TypeDef
            use writ_module::tables::TableId;
            let target_row = (typedef_idx + 1) as u32;
            let is_generic = module.module.generic_params.iter().any(|p|
                p.owner.table_id() == TableId::TypeDef.as_u8()
                && p.owner.row_index() == Some(target_row)
            );
            let _ = heap.set_field(href, 3, Value::Bool(is_generic));

            // Field 4: type_args (Array<Type>) — empty for non-TypeSpec types
            let empty_arr = heap.alloc_array(0);
            let _ = heap.set_field(href, 4, Value::Ref(empty_arr));
        }

        self.type_cache.insert(key, href);
        self.type_reverse.insert(href, key);
        href
    }

    /// Get or allocate a Type heap object for a primitive type (Int, Float, Bool, String).
    ///
    /// Uses synthetic cache keys `(usize::MAX, ordinal)` where ordinals are:
    /// Int=0, Float=1, Bool=2, String=3
    pub fn get_or_alloc_primitive_type(
        &mut self,
        name: &str,
        heap: &mut dyn GcHeap,
    ) -> HeapRef {
        let ordinal = match name {
            "Int" => 0,
            "Float" => 1,
            "Bool" => 2,
            "String" => 3,
            _ => usize::MAX - 1, // fallback for unknown primitives
        };
        let key = (usize::MAX, ordinal);

        if let Some(&href) = self.type_cache.get(&key) {
            return href;
        }

        // Allocate the Type class object for this primitive
        let href = heap.alloc_struct(Self::TYPE_TYPE_KEY, Self::TYPE_FIELD_COUNT);

        // Field 0: name (string) — the primitive name
        let name_href = heap.alloc_string(name);
        let _ = heap.set_field(href, 0, Value::Ref(name_href));

        // Field 1: namespace (string) — empty for primitives
        let ns_href = heap.alloc_string("");
        let _ = heap.set_field(href, 1, Value::Ref(ns_href));

        // Field 2: kind (string) — "primitive"
        let kind_href = heap.alloc_string("primitive");
        let _ = heap.set_field(href, 2, Value::Ref(kind_href));

        // Field 3: is_generic (bool) — false (primitives are never generic)
        let _ = heap.set_field(href, 3, Value::Bool(false));

        // Field 4: type_args (Array<Type>) — empty for primitives
        let empty_arr = heap.alloc_array(0);
        let _ = heap.set_field(href, 4, Value::Ref(empty_arr));

        self.type_cache.insert(key, href);
        self.type_reverse.insert(href, key);
        href
    }

    /// Collect all cached HeapRefs as GC roots.
    ///
    /// Called from `Runtime::collect_roots()` to prevent the GC from freeing
    /// any cached reflection objects that have no script-side references.
    /// The reverse maps (type_reverse, field_reverse, method_reverse) hold the same HeapRefs as
    /// the forward caches so they are already covered.
    pub fn collect_roots(&self, out: &mut Vec<HeapRef>) {
        out.extend(self.type_cache.values().copied());
        out.extend(self.field_cache.values().copied());
        out.extend(self.method_cache.values().copied());
        out.extend(self.param_cache.values().copied());
        out.extend(self.attr_cache.values().copied());
        out.extend(self.contract_info_cache.values().copied());
        out.extend(self.method_attr_cache.values().copied());
        out.extend(self.field_attr_cache.values().copied());
    }

    // ── Helper allocation methods ─────────────────────────────────

    /// The type_key for FieldInfo heap objects (virtual module index 0, TypeDef index 13).
    const FIELD_INFO_TYPE_KEY: u32 = (0u32 << 16) | 13u32;
    /// The type_key for MethodInfo heap objects (virtual module index 0, TypeDef index 14).
    const METHOD_INFO_TYPE_KEY: u32 = (0u32 << 16) | 14u32;
    /// The type_key for AttributeInfo heap objects (virtual module index 0, TypeDef index 11).
    const ATTR_INFO_TYPE_KEY: u32 = (0u32 << 16) | 11u32;
    /// The type_key for ContractInfo heap objects (virtual module index 0, TypeDef index 12).
    const CONTRACT_INFO_TYPE_KEY: u32 = (0u32 << 16) | 12u32;

    /// Get the range of field indices for a TypeDef (0-based start inclusive, exclusive end).
    ///
    /// Returns `(field_start, field_end)` where `field_start..field_end` indexes into
    /// `modules[module_idx].module.field_defs`.
    pub fn typedef_field_range_pub(modules: &[LoadedModule], module_idx: usize, typedef_idx: usize)
        -> (usize, usize)
    {
        if module_idx >= modules.len() {
            return (0, 0);
        }
        let module = &modules[module_idx].module;
        if typedef_idx >= module.type_defs.len() {
            return (0, 0);
        }
        let td = &module.type_defs[typedef_idx];
        let start = td.field_list.saturating_sub(1) as usize;
        let end = if typedef_idx + 1 < module.type_defs.len() {
            module.type_defs[typedef_idx + 1].field_list.saturating_sub(1) as usize
        } else {
            module.field_defs.len()
        };
        (start, end)
    }

    /// Get the range of method indices for a TypeDef (0-based start inclusive, exclusive end).
    ///
    /// Returns `(method_start, method_end)` where `method_start..method_end` indexes into
    /// `modules[module_idx].module.method_defs`.
    pub fn typedef_method_range_pub(modules: &[LoadedModule], module_idx: usize, typedef_idx: usize)
        -> (usize, usize)
    {
        if module_idx >= modules.len() {
            return (0, 0);
        }
        let module = &modules[module_idx].module;
        if typedef_idx >= module.type_defs.len() {
            return (0, 0);
        }
        let td = &module.type_defs[typedef_idx];
        let start = td.method_list.saturating_sub(1) as usize;
        let end = if typedef_idx + 1 < module.type_defs.len() {
            module.type_defs[typedef_idx + 1].method_list.saturating_sub(1) as usize
        } else {
            module.method_defs.len()
        };
        (start, end)
    }

    /// Get or allocate a FieldInfo heap object for the given field.
    ///
    /// Cache key: (module_idx, field_0based_idx) — absolute field index in the module.
    /// The `field_offset` is the 0-based offset within the typedef's field range.
    pub fn get_or_alloc_field_info(
        &mut self,
        module_idx: usize,
        typedef_idx: usize,
        field_offset: usize,
        heap: &mut dyn GcHeap,
        modules: &[LoadedModule],
    ) -> HeapRef {
        let (field_start, _field_end) = Self::typedef_field_range_pub(modules, module_idx, typedef_idx);
        let absolute_field_idx = field_start + field_offset;
        let key = (module_idx, absolute_field_idx);

        if let Some(&href) = self.field_cache.get(&key) {
            return href;
        }

        let href = heap.alloc_struct(Self::FIELD_INFO_TYPE_KEY, 3);

        let module = &modules[module_idx].module;
        if absolute_field_idx < module.field_defs.len() {
            let fd = &module.field_defs[absolute_field_idx];

            // Field 0 (name): string from field def
            let name = read_string(&module.string_heap, fd.name).unwrap_or("").to_owned();
            let name_href = heap.alloc_string(&name);
            let _ = heap.set_field(href, 0, Value::Ref(name_href));

            // Field 1 (declared_type): Value::Void placeholder (full type resolution in Phase 106)
            let _ = heap.set_field(href, 1, Value::Void);

            // Field 2 (is_mutable): 0x01 = FIELD_FLAG_READONLY; is_mutable = (flags & 0x01) == 0
            let is_mutable = (fd.flags & 0x01) == 0;
            let _ = heap.set_field(href, 2, Value::Bool(is_mutable));
        }

        self.field_cache.insert(key, href);
        self.field_reverse.insert(href, (module_idx, typedef_idx, field_offset));
        href
    }

    /// Get the field identity from a FieldInfo HeapRef.
    ///
    /// Returns `(module_idx, typedef_idx, field_offset)` or None if not in the reverse map.
    pub fn lookup_field_identity(&self, href: HeapRef) -> Option<(usize, usize, usize)> {
        self.field_reverse.get(&href).copied()
    }

    /// Get or allocate a MethodInfo heap object for the given method.
    ///
    /// Cache key: (module_idx, method_0based_idx) — absolute method index in the module.
    pub fn get_or_alloc_method_info(
        &mut self,
        module_idx: usize,
        method_idx: usize,
        heap: &mut dyn GcHeap,
        modules: &[LoadedModule],
    ) -> HeapRef {
        let key = (module_idx, method_idx);

        if let Some(&href) = self.method_cache.get(&key) {
            return href;
        }

        let href = heap.alloc_struct(Self::METHOD_INFO_TYPE_KEY, 3);

        let module = &modules[module_idx].module;
        if method_idx < module.method_defs.len() {
            let md = &module.method_defs[method_idx];

            // Field 0 (name): string from method def
            let name = read_string(&module.string_heap, md.name).unwrap_or("").to_owned();
            let name_href = heap.alloc_string(&name);
            let _ = heap.set_field(href, 0, Value::Ref(name_href));

            // Field 1 (return_type): Value::Void placeholder (full type resolution in Phase 106)
            let _ = heap.set_field(href, 1, Value::Void);

            // Field 2 (parameters): empty Array (full param population in Phase 106)
            let arr_href = heap.alloc_array(0); // elem_type=0 (void/untyped)
            let _ = heap.set_field(href, 2, Value::Ref(arr_href));
        }

        self.method_cache.insert(key, href);
        self.method_reverse.insert(href, (module_idx, method_idx));
        href
    }

    /// Get the method identity from a MethodInfo HeapRef.
    ///
    /// Returns `(module_idx, method_idx)` or None if not in the reverse map.
    pub fn lookup_method_identity(&self, href: HeapRef) -> Option<(usize, usize)> {
        self.method_reverse.get(&href).copied()
    }

    /// Get or allocate an AttributeInfo heap object for the given attribute occurrence.
    ///
    /// Cache key: (module_idx, typedef_idx, ordinal) where ordinal is the 0-based index
    /// within the attribute list for the given TypeDef.
    pub fn get_or_alloc_attribute_info(
        &mut self,
        name: &str,
        args: &[writ_module::attr::AttrValue],
        ordinal: usize,
        module_idx: usize,
        typedef_idx: usize,
        heap: &mut dyn GcHeap,
    ) -> HeapRef {
        let key = (module_idx, typedef_idx, ordinal);

        if let Some(&href) = self.attr_cache.get(&key) {
            return href;
        }

        let href = self.allocate_attribute_info(name, args, heap);
        self.attr_cache.insert(key, href);
        href
    }

    /// Shared helper: allocate a new AttributeInfo heap object without caching.
    ///
    /// Used by `get_or_alloc_attribute_info`, `get_or_alloc_method_attribute_info`,
    /// and `get_or_alloc_field_attribute_info` to avoid code duplication.
    fn allocate_attribute_info(
        &mut self,
        name: &str,
        args: &[writ_module::attr::AttrValue],
        heap: &mut dyn GcHeap,
    ) -> HeapRef {
        let href = heap.alloc_struct(Self::ATTR_INFO_TYPE_KEY, 2);

        // Field 0 (name): attribute name string
        let name_href = heap.alloc_string(name);
        let _ = heap.set_field(href, 0, Value::Ref(name_href));

        // Field 1 (args): Array of boxed AttrValues
        let arr_href = heap.alloc_array(0);
        for arg in args {
            let boxed = match arg {
                writ_module::attr::AttrValue::String(s) => {
                    let s_href = heap.alloc_string(s);
                    heap.alloc_boxed(Value::Ref(s_href))
                }
                writ_module::attr::AttrValue::Int(i) => {
                    heap.alloc_boxed(Value::Int(*i))
                }
                writ_module::attr::AttrValue::Bool(b) => {
                    heap.alloc_boxed(Value::Bool(*b))
                }
                writ_module::attr::AttrValue::Named { value, .. } => {
                    match value.as_ref() {
                        writ_module::attr::AttrValue::String(s) => {
                            let s_href = heap.alloc_string(s);
                            heap.alloc_boxed(Value::Ref(s_href))
                        }
                        writ_module::attr::AttrValue::Int(i) => heap.alloc_boxed(Value::Int(*i)),
                        writ_module::attr::AttrValue::Bool(b) => heap.alloc_boxed(Value::Bool(*b)),
                        _ => heap.alloc_boxed(Value::Void),
                    }
                }
            };
            if let Ok(HeapObject::Array { elements, .. }) = heap.get_object_mut(arr_href) {
                elements.push(Value::Ref(boxed));
            }
        }
        let _ = heap.set_field(href, 1, Value::Ref(arr_href));
        href
    }

    /// Get or allocate an AttributeInfo heap object for a method attribute.
    ///
    /// Cache key: (module_idx, method_idx, ordinal).
    pub fn get_or_alloc_method_attribute_info(
        &mut self,
        name: &str,
        args: &[writ_module::attr::AttrValue],
        ordinal: usize,
        module_idx: usize,
        method_idx: usize,
        heap: &mut dyn GcHeap,
    ) -> HeapRef {
        let key = (module_idx, method_idx, ordinal);
        if let Some(&href) = self.method_attr_cache.get(&key) {
            return href;
        }
        let href = self.allocate_attribute_info(name, args, heap);
        self.method_attr_cache.insert(key, href);
        href
    }

    /// Get or allocate an AttributeInfo heap object for a field attribute.
    ///
    /// Cache key: (module_idx, field_idx, ordinal).
    pub fn get_or_alloc_field_attribute_info(
        &mut self,
        name: &str,
        args: &[writ_module::attr::AttrValue],
        ordinal: usize,
        module_idx: usize,
        field_idx: usize,
        heap: &mut dyn GcHeap,
    ) -> HeapRef {
        let key = (module_idx, field_idx, ordinal);
        if let Some(&href) = self.field_attr_cache.get(&key) {
            return href;
        }
        let href = self.allocate_attribute_info(name, args, heap);
        self.field_attr_cache.insert(key, href);
        href
    }

    /// Get or allocate a Type heap object for a TypeSpec (instantiated generic type).
    ///
    /// Resolves the underlying TypeDef from the TypeSpec signature, then allocates
    /// a Type with is_generic=true and type_args populated from the signature blob.
    ///
    /// Cache key: (module_idx, usize::MAX - 1 - typespec_idx) to avoid collision with typedef keys.
    pub fn get_or_alloc_typespec_type(
        &mut self,
        module_idx: usize,
        typespec_idx: usize,
        heap: &mut dyn GcHeap,
        modules: &[LoadedModule],
    ) -> HeapRef {
        let cache_key = (module_idx, usize::MAX - 1 - typespec_idx);
        if let Some(&href) = self.type_cache.get(&cache_key) {
            return href;
        }

        let href = heap.alloc_struct(Self::TYPE_TYPE_KEY, Self::TYPE_FIELD_COUNT);

        // Read the TypeSpec signature blob
        let sig_blob: Vec<u8> = {
            let module = &modules[module_idx].module;
            if typespec_idx < module.type_specs.len() {
                let ts = &module.type_specs[typespec_idx];
                match writ_module::heap::read_blob(&module.blob_heap, ts.signature) {
                    Ok(b) => b.to_vec(),
                    Err(_) => Vec::new(),
                }
            } else {
                Vec::new()
            }
        };

        let mut type_arg_hrefs: Vec<Value> = Vec::new();

        if sig_blob.len() >= 2 && sig_blob[0] == 0x20 {
            // Array<T> instantiation — base type is "Array" from virtual module
            let name_href = heap.alloc_string("Array");
            let _ = heap.set_field(href, 0, Value::Ref(name_href));
            let ns_href = heap.alloc_string("writ");
            let _ = heap.set_field(href, 1, Value::Ref(ns_href));
            let kind_href = heap.alloc_string("class");
            let _ = heap.set_field(href, 2, Value::Ref(kind_href));

            // Extract element type from sig_blob[1..]
            let elem_tag = sig_blob[1];
            let elem_type_href = match elem_tag {
                0x01 => self.get_or_alloc_primitive_type("Bool", heap),
                0x02 => self.get_or_alloc_primitive_type("Int", heap),
                0x03 => self.get_or_alloc_primitive_type("Float", heap),
                0x04 => self.get_or_alloc_primitive_type("String", heap),
                0x10 if sig_blob.len() >= 6 => {
                    // TypeRef: next 4 bytes are a metadata token
                    let token_val = u32::from_le_bytes([sig_blob[2], sig_blob[3], sig_blob[4], sig_blob[5]]);
                    let typedef_0based = ((token_val & 0x00FF_FFFF) as usize).saturating_sub(1);
                    let type_module_idx = if (token_val >> 24) == 2 { module_idx } else { 0 };
                    self.get_or_alloc_type(type_module_idx, typedef_0based, heap, modules)
                }
                _ => self.get_or_alloc_primitive_type("Int", heap), // fallback
            };
            type_arg_hrefs.push(Value::Ref(elem_type_href));
        } else {
            // Generic non-Array TypeSpec — use name "Unknown"
            let name_href = heap.alloc_string("Unknown");
            let _ = heap.set_field(href, 0, Value::Ref(name_href));
            let ns_href = heap.alloc_string("");
            let _ = heap.set_field(href, 1, Value::Ref(ns_href));
            let kind_href = heap.alloc_string("class");
            let _ = heap.set_field(href, 2, Value::Ref(kind_href));
        }

        // Field 3: is_generic = true for all TypeSpecs
        let _ = heap.set_field(href, 3, Value::Bool(true));

        // Field 4: type_args array
        let arr_href = heap.alloc_array(0);
        if let Ok(HeapObject::Array { elements, .. }) = heap.get_object_mut(arr_href) {
            for v in type_arg_hrefs {
                elements.push(v);
            }
        }
        let _ = heap.set_field(href, 4, Value::Ref(arr_href));

        self.type_cache.insert(cache_key, href);
        self.type_reverse.insert(href, cache_key);
        href
    }

    /// Get or allocate a ContractInfo heap object for the given ContractDef.
    ///
    /// Cache key: (module_idx, contract_0based_idx).
    pub fn get_or_alloc_contract_info(
        &mut self,
        module_idx: usize,
        contract_idx: usize,
        heap: &mut dyn GcHeap,
        modules: &[LoadedModule],
    ) -> HeapRef {
        let key = (module_idx, contract_idx);

        if let Some(&href) = self.contract_info_cache.get(&key) {
            return href;
        }

        let href = heap.alloc_struct(Self::CONTRACT_INFO_TYPE_KEY, 2);

        let module = &modules[module_idx].module;
        if contract_idx < module.contract_defs.len() {
            let cd = &module.contract_defs[contract_idx];

            // Field 0 (name): string from contract def
            let name = read_string(&module.string_heap, cd.name).unwrap_or("").to_owned();
            let name_href = heap.alloc_string(&name);
            let _ = heap.set_field(href, 0, Value::Ref(name_href));

            // Field 1 (type): Value::Void placeholder (full Type cross-ref in Phase 106)
            let _ = heap.set_field(href, 1, Value::Void);
        }

        self.contract_info_cache.insert(key, href);
        href
    }

    /// Look up the (module_idx, typedef_idx) for a Type heap object.
    ///
    /// Returns None if the HeapRef was not allocated by this ReflectionIndex
    /// or represents a primitive type (module_idx == usize::MAX).
    pub fn lookup_type_identity(&self, href: HeapRef) -> Option<(usize, usize)> {
        self.type_reverse.get(&href).copied()
    }
}

impl Default for ReflectionIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gc::MarkSweepHeap;

    #[test]
    fn primitive_type_alloc_and_cache() {
        let mut idx = ReflectionIndex::new();
        let mut heap = MarkSweepHeap::new();

        let href1 = idx.get_or_alloc_primitive_type("Int", &mut heap);
        let href2 = idx.get_or_alloc_primitive_type("Int", &mut heap);
        // Same cached object returned on second call
        assert_eq!(href1.0, href2.0, "cached HeapRef should be returned on second call");

        // Different primitives get different objects
        let float_href = idx.get_or_alloc_primitive_type("Float", &mut heap);
        assert_ne!(href1.0, float_href.0, "Int and Float should have separate Type objects");
    }

    #[test]
    fn collect_roots_includes_all_cached_refs() {
        let mut idx = ReflectionIndex::new();
        let mut heap = MarkSweepHeap::new();

        let int_href = idx.get_or_alloc_primitive_type("Int", &mut heap);
        let float_href = idx.get_or_alloc_primitive_type("Float", &mut heap);

        let mut roots = Vec::new();
        idx.collect_roots(&mut roots);

        assert!(roots.contains(&int_href));
        assert!(roots.contains(&float_href));
    }

    #[test]
    fn primitive_type_survives_gc() {
        let mut idx = ReflectionIndex::new();
        let mut heap = MarkSweepHeap::new();

        let int_href = idx.get_or_alloc_primitive_type("Int", &mut heap);

        // Collect roots and run GC — the Type object should survive
        let mut roots = Vec::new();
        idx.collect_roots(&mut roots);
        let stats = heap.collect(&roots);

        // The Type object (5 fields struct) plus 3 string fields plus 1 empty Array (field 4)
        // = 5 heap objects total, all survive since transitively reachable from the root
        assert_eq!(stats.objects_freed, 0, "cached Type object should survive GC");
        assert!(heap.get_object(int_href).is_ok(), "Type object still accessible after GC");
    }
}
