use crate::num_traits::FromPrimitive;
use crate::types;
use std::ffi::CStr;
use std::os::raw::c_char;

pub const STARTING_WORD: usize = 5;
pub const SPIRV_WORD_SIZE: usize = std::mem::size_of::<u32>();
pub const SPIRV_BYTE_WIDTH: usize = 8;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(crate) struct NumberDecoration {
    pub word_offset: u32,
    pub value: u32,
}

impl Default for NumberDecoration {
    fn default() -> NumberDecoration {
        Self {
            word_offset: 0,
            value: std::u32::MAX,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, PartialEq)]
pub(crate) struct StringDecoration {
    pub word_offset: u32,
    pub value: String,
}

#[derive(Default, Debug, Clone, PartialEq)]
pub(crate) struct Decorations {
    pub is_block: bool,
    pub is_buffer_block: bool,
    pub is_row_major: bool,
    pub is_column_major: bool,
    //pub is_built_in: bool,
    pub is_noperspective: bool,
    pub is_flat: bool,
    pub is_non_writable: bool,
    pub set: NumberDecoration,
    pub binding: NumberDecoration,
    pub input_attachment_index: NumberDecoration,
    pub location: NumberDecoration,
    pub offset: NumberDecoration,
    pub uav_counter_buffer: NumberDecoration,
    pub semantic: StringDecoration,
    pub array_stride: u32,
    pub matrix_stride: u32,
    pub built_in: Option<spirv_headers::BuiltIn>,
}

#[derive(Default, Debug, Clone, PartialEq)]
pub(crate) struct ParserArrayTraits {
    pub element_type_id: u32,
    pub length_id: u32,
}

#[derive(Default, Debug, Clone, PartialEq)]
pub(crate) struct ParserImageTraits {
    pub sampled_type_id: u32,
    pub dim: Option<spirv_headers::Dim>,
    pub depth: u32,
    pub arrayed: u32,
    pub ms: u32,
    pub sampled: u32,
    pub image_format: Option<spirv_headers::ImageFormat>,
}

/*
impl Default for ParserImageTraits {
    fn default() -> Self {
        ParserImageTraits {
            sampled_type_id: 0,
            dim: spirv_headers::Dim::Dim1D,
            depth: 0,
            arrayed: 0,
            ms: 0,
            sampled: 0,
            image_format: spirv_headers::ImageFormat::Unknown,
        }
    }
}
*/

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ParserNode {
    pub result_id: u32,
    pub op: spirv_headers::Op,
    pub result_type_id: u32,
    pub type_id: u32,
    pub storage_class: spirv_headers::StorageClass,
    pub word_offset: u32,
    pub word_count: u32,
    pub is_type: bool,
    pub array_traits: ParserArrayTraits,
    pub image_traits: ParserImageTraits,
    pub image_type_id: u32,
    pub name: String,
    pub decorations: Decorations,
    pub member_count: u32,
    pub member_names: Vec<String>,
    pub member_decorations: Vec<Decorations>,
}

impl Default for ParserNode {
    fn default() -> ParserNode {
        Self {
            result_id: 0,
            op: spirv_headers::Op::Undef,
            result_type_id: 0,
            type_id: 0,
            storage_class: spirv_headers::StorageClass::UniformConstant,
            word_offset: 0,
            word_count: 0,
            is_type: false,
            array_traits: ParserArrayTraits::default(),
            image_traits: ParserImageTraits::default(),
            image_type_id: 0,
            name: String::new(),
            decorations: Decorations::default(),
            member_count: 0,
            member_names: Vec::new(),
            member_decorations: Vec::new(),
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq)]
pub(crate) struct ParserFunctionCallee {
    pub callee: u32,
    pub function: usize,
}

#[derive(Default, Debug, Clone, PartialEq)]
pub(crate) struct ParserFunction {
    pub id: u32,
    pub callees: Vec<ParserFunctionCallee>,
    pub accessed: Vec<u32>,
}

#[derive(Default, Debug)]
pub(crate) struct ParserString {
    pub result_id: u32,
    pub string: String,
}

#[derive(Default, Debug)]
pub(crate) struct Parser {
    pub nodes: Vec<ParserNode>,
    pub strings: Vec<ParserString>,
    pub functions: Vec<ParserFunction>,

    pub string_count: usize,
    pub type_count: usize,
    pub descriptor_count: usize,
    pub push_constant_count: usize,
    pub entry_point_count: usize,
    pub function_count: usize,
}

impl Parser {
    pub(crate) fn parse(
        &mut self,
        spv_words: &[u32],
        module: &mut super::ShaderModule,
    ) -> Result<(), String> {
        if spv_words.len() == 0 {
            return Err("No SPIR-V specified for shader module".to_string());
        }

        if spv_words[0] != spirv_headers::MAGIC_NUMBER {
            return Err("Invalid SPIR-V - does not start with valid magic number.".to_string());
        }

        // Determine generator
        let generator: u32 = (spv_words[2] & 0xFFFF0000) >> 16u32;
        let _generator = match generator {
            6 => types::ReflectGenerator::KhronosLlvmSpirvTranslator,
            7 => types::ReflectGenerator::KhronosSpirvToolsAssembler,
            8 => types::ReflectGenerator::KhronosGlslangReferenceFrontEnd,
            13 => types::ReflectGenerator::GoogleShadercOverGlslang,
            14 => types::ReflectGenerator::GoogleSpiregg,
            15 => types::ReflectGenerator::GoogleRspirv,
            16 => types::ReflectGenerator::XLegendMesaMesairSpirvTranslator,
            17 => types::ReflectGenerator::KhronosSpirvToolsLinker,
            18 => types::ReflectGenerator::WineVkd3dShaderCompiler,
            19 => types::ReflectGenerator::ClayClayShaderCompiler,
            _ => types::ReflectGenerator::Unknown,
        };

        self.parse_nodes(spv_words, module)?;
        self.parse_strings(spv_words, module)?;
        self.parse_functions(spv_words, module)?;
        self.parse_member_counts(spv_words, module)?;
        self.parse_names(spv_words, module)?;
        self.parse_decorations(spv_words, module)?;
        self.parse_types(spv_words, module)?;
        self.parse_descriptor_bindings(spv_words, module)?;
        self.parse_descriptor_type(module)?;
        self.parse_counter_bindings(spv_words, module)?;
        self.parse_descriptor_blocks(spv_words, module)?;
        self.parse_push_constant_blocks(spv_words, module)?;
        self.parse_entry_points(spv_words, module)?;

        // Fix up SRV vs UAV descriptors for storage buffers
        for mut descriptor_binding in &mut module.internal.descriptor_bindings {
            if descriptor_binding.descriptor_type
                == crate::types::ReflectDescriptorType::StorageBuffer
                && descriptor_binding
                    .block
                    .decoration_flags
                    .contains(crate::types::ReflectDecorationFlags::NON_WRITABLE)
            {
                descriptor_binding.resource_type =
                    crate::types::ReflectResourceTypeFlags::SHADER_RESOURCE_VIEW;
            }
        }

        module.internal.build_descriptor_sets()?;

        // TODO: Clean this up
        if module.internal.entry_points.len() > 0 {
            let entry_point = &module.internal.entry_points[0];
            module.internal.entry_point_name = entry_point.name.to_owned();
            module.internal.entry_point_id = entry_point.id;
            module.internal.spirv_execution_model = entry_point.spirv_execution_model;
            module.internal.shader_stage = entry_point.shader_stage;
            module.internal.input_variables = entry_point.input_variables.clone();
            module.internal.output_variables = entry_point.output_variables.clone();
        }

        Ok(())
    }

    fn parse_nodes(
        &mut self,
        spv_words: &[u32],
        module: &mut super::ShaderModule,
    ) -> Result<(), String> {
        let mut word_index = STARTING_WORD;

        // Count the nodes
        let mut node_count = 0usize;
        while word_index < spv_words.len() {
            let word = spv_words[word_index];
            let node_word_count = (word >> 16u32) & 0xFFFF;
            word_index += node_word_count as usize;
            node_count += 1;
        }

        self.nodes.resize(node_count, ParserNode::default());

        if self.nodes.len() == 0 {
            return Err("No nodes found in SPIR-V binary, invalid!".to_string());
        }

        // Restart parser and process nodes
        word_index = STARTING_WORD;

        let mut function_node: usize = std::usize::MAX;
        let mut node_index = 0;

        while word_index < spv_words.len() {
            let word = spv_words[word_index];
            {
                let mut node = &mut self.nodes[node_index];
                node.word_count = (word >> 16u32) & 0x0000FFFF;
                node.word_offset = word_index as u32;
                if let Some(op) = spirv_headers::Op::from_u32(word & 0x0000FFFF) {
                    node.op = op;
                } else {
                    return Err(format!("Invalid SPIR-V op: {}", word & 0x0000FFFF));
                }
            }

            match self.nodes[node_index].op {
                spirv_headers::Op::String => self.string_count += 1,
                spirv_headers::Op::Source => {
                    module.internal.source_language =
                        spirv_headers::SourceLanguage::from_u32(spv_words[word_index + 1]);
                    module.internal.source_language_version = spv_words[word_index + 2];
                    if self.nodes[node_index].word_count >= 4 {
                        module.internal.source_file_id = spv_words[word_index + 3];
                    }
                }
                spirv_headers::Op::EntryPoint => self.entry_point_count += 1,
                spirv_headers::Op::Name | spirv_headers::Op::MemberName => {
                    let member_offset: usize =
                        if self.nodes[node_index].op == spirv_headers::Op::MemberName {
                            1
                        } else {
                            0
                        };
                    let name_start = word_index + member_offset + 2;
                    let mut node = &mut self.nodes[node_index];
                    node.name = unsafe {
                        let name_ptr =
                            spv_words.as_ptr().offset(name_start as isize) as *const c_char;
                        let name_str = CStr::from_ptr(name_ptr);
                        name_str.to_str().unwrap().to_owned()
                    };
                }
                spirv_headers::Op::TypeStruct => {
                    let mut node = &mut self.nodes[node_index];
                    node.member_count = node.word_count - 2;
                    node.result_id = spv_words[word_index + 1];
                    node.is_type = true;
                }
                spirv_headers::Op::TypeVoid
                | spirv_headers::Op::TypeBool
                | spirv_headers::Op::TypeInt
                | spirv_headers::Op::TypeFloat
                | spirv_headers::Op::TypeVector
                | spirv_headers::Op::TypeMatrix
                | spirv_headers::Op::TypeSampler
                | spirv_headers::Op::TypeOpaque
                | spirv_headers::Op::TypeFunction
                | spirv_headers::Op::TypeEvent
                | spirv_headers::Op::TypeDeviceEvent
                | spirv_headers::Op::TypeReserveId
                | spirv_headers::Op::TypeQueue
                | spirv_headers::Op::TypePipe
                | spirv_headers::Op::TypeAccelerationStructureNV => {
                    let mut node = &mut self.nodes[node_index];
                    node.result_id = spv_words[word_index + 1];
                    node.is_type = true;
                }
                spirv_headers::Op::TypeImage => {
                    let mut node = &mut self.nodes[node_index];
                    node.result_id = spv_words[word_index + 1];
                    node.image_traits.sampled_type_id = spv_words[word_index + 2];
                    node.image_traits.dim = spirv_headers::Dim::from_u32(spv_words[word_index + 3]);
                    node.image_traits.depth = spv_words[word_index + 4];
                    node.image_traits.arrayed = spv_words[word_index + 5];
                    node.image_traits.ms = spv_words[word_index + 6];
                    node.image_traits.sampled = spv_words[word_index + 7];
                    node.image_traits.image_format =
                        spirv_headers::ImageFormat::from_u32(spv_words[word_index + 8]);
                    node.is_type = true;
                }
                spirv_headers::Op::TypeSampledImage => {
                    let mut node = &mut self.nodes[node_index];
                    node.result_id = spv_words[word_index + 1];
                    node.image_type_id = spv_words[word_index + 2];
                    node.is_type = true;
                }
                spirv_headers::Op::TypeArray => {
                    let mut node = &mut self.nodes[node_index];
                    node.result_id = spv_words[word_index + 1];
                    node.array_traits.element_type_id = spv_words[word_index + 2];
                    node.array_traits.length_id = spv_words[word_index + 3];
                    node.is_type = true;
                }
                spirv_headers::Op::TypeRuntimeArray => {
                    let mut node = &mut self.nodes[node_index];
                    node.result_id = spv_words[word_index + 1];
                    node.array_traits.element_type_id = spv_words[word_index + 2];
                    node.is_type = true;
                }
                spirv_headers::Op::TypePointer => {
                    let mut node = &mut self.nodes[node_index];
                    node.result_id = spv_words[word_index + 1];
                    if let Some(storage_class) =
                        spirv_headers::StorageClass::from_u32(spv_words[word_index + 2])
                    {
                        node.storage_class = storage_class;
                    } else {
                        return Err("Invalid SPIR-V storage class!".into());
                    }
                    node.type_id = spv_words[word_index + 3];
                    node.is_type = true;
                }
                spirv_headers::Op::TypeForwardPointer => {
                    let mut node = &mut self.nodes[node_index];
                    node.result_id = spv_words[word_index + 1];
                    if let Some(storage_class) =
                        spirv_headers::StorageClass::from_u32(spv_words[word_index + 2])
                    {
                        node.storage_class = storage_class;
                    } else {
                        return Err("Invalid SPIR-V storage class!".into());
                    }
                    node.is_type = true;
                }
                spirv_headers::Op::ConstantTrue
                | spirv_headers::Op::ConstantFalse
                | spirv_headers::Op::Constant
                | spirv_headers::Op::ConstantComposite
                | spirv_headers::Op::ConstantSampler
                | spirv_headers::Op::ConstantNull => {
                    let mut node = &mut self.nodes[node_index];
                    node.result_type_id = spv_words[word_index + 1];
                    node.result_id = spv_words[word_index + 2];
                }
                spirv_headers::Op::Variable => {
                    let mut node = &mut self.nodes[node_index];
                    node.type_id = spv_words[word_index + 1];
                    node.result_id = spv_words[word_index + 2];
                    if let Some(storage_class) =
                        spirv_headers::StorageClass::from_u32(spv_words[word_index + 3])
                    {
                        node.storage_class = storage_class;
                    } else {
                        return Err("Invalid SPIR-V storage class!".into());
                    }
                }
                spirv_headers::Op::Load => {
                    let mut node = &mut self.nodes[node_index];
                    node.result_type_id = spv_words[word_index + 1];
                    node.result_id = spv_words[word_index + 2];
                }
                spirv_headers::Op::Function => {
                    let mut node = &mut self.nodes[node_index];
                    node.result_id = spv_words[word_index + 2];
                    function_node = node_index;
                }
                spirv_headers::Op::Label => {
                    if function_node != std::usize::MAX {
                        let mut node = &mut self.nodes[function_node];
                        node.result_id = spv_words[node.word_offset as usize + 2];
                        self.function_count += 1;
                    }
                }
                spirv_headers::Op::FunctionEnd => function_node = std::usize::MAX,
                _ => {}
            }

            let node = &self.nodes[node_index];
            if node.is_type {
                self.type_count += 1;
            }
            word_index += node.word_count as usize;
            node_index += 1;
        }

        Ok(())
    }

    fn parse_strings(
        &mut self,
        spv_words: &[u32],
        module: &mut super::ShaderModule,
    ) -> Result<(), String> {
        if self.string_count > 0 && spv_words.len() > 0 && self.nodes.len() > 0 {
            self.strings.reserve(self.string_count);
            for node in &self.nodes {
                if node.op != spirv_headers::Op::String {
                    continue;
                }

                if self.strings.len() >= self.string_count {
                    return Err("Count mismatch while parsing strings.".into());
                }

                let string_start = node.word_offset as usize + 2;
                let string_value = unsafe {
                    let string_ptr =
                        spv_words.as_ptr().offset(string_start as isize) as *const c_char;
                    let string_str = CStr::from_ptr(string_ptr);
                    string_str.to_str().unwrap().to_owned()
                };

                self.strings.push(ParserString {
                    result_id: spv_words[node.word_offset as usize + 1],
                    string: string_value,
                });
            }

            for string in &self.strings {
                if string.result_id == module.internal.source_file_id {
                    module.internal.source_file = string.string.to_owned();
                    break;
                }
            }
        }

        Ok(())
    }

    fn parse_function(
        &mut self,
        spv_words: &[u32],
        //_module: &mut super::ShaderModule,
        function_node_index: usize,
        first_label_index: usize,
    ) -> Result<ParserFunction, String> {
        let mut function = ParserFunction {
            id: self.nodes[function_node_index].result_id,
            callees: Vec::new(),
            accessed: Vec::new(),
        };

        let mut callee_count = 0;
        let mut accessed_count = 0;

        for node_index in first_label_index..self.nodes.len() {
            let node_op = self.nodes[node_index].op;
            if node_op != spirv_headers::Op::FunctionEnd {
                continue;
            }

            match node_op {
                spirv_headers::Op::FunctionCall => {
                    callee_count += 1;
                }
                spirv_headers::Op::Load
                | spirv_headers::Op::AccessChain
                | spirv_headers::Op::InBoundsAccessChain
                | spirv_headers::Op::PtrAccessChain
                | spirv_headers::Op::ArrayLength
                | spirv_headers::Op::GenericPtrMemSemantics
                | spirv_headers::Op::InBoundsPtrAccessChain
                | spirv_headers::Op::Store => {
                    accessed_count += 1;
                }
                spirv_headers::Op::CopyMemory | spirv_headers::Op::CopyMemorySized => {
                    accessed_count += 2;
                }
                _ => {}
            }
        }

        function.callees.reserve(callee_count);
        function.accessed.reserve(accessed_count);

        for node_index in first_label_index..self.nodes.len() {
            let node_op = self.nodes[node_index].op;
            if node_op != spirv_headers::Op::FunctionEnd {
                continue;
            }

            let word_offset = self.nodes[node_index].word_offset as usize;
            match node_op {
                spirv_headers::Op::FunctionCall => {
                    function.callees.push(ParserFunctionCallee {
                        callee: spv_words[word_offset + 3],
                        function: std::usize::MAX, // resolved later
                    });
                }
                spirv_headers::Op::Load
                | spirv_headers::Op::AccessChain
                | spirv_headers::Op::InBoundsAccessChain
                | spirv_headers::Op::PtrAccessChain
                | spirv_headers::Op::ArrayLength
                | spirv_headers::Op::GenericPtrMemSemantics
                | spirv_headers::Op::InBoundsPtrAccessChain => {
                    function.accessed.push(spv_words[word_offset + 3]);
                }
                spirv_headers::Op::Store => {
                    function.accessed.push(spv_words[word_offset + 2]);
                }
                spirv_headers::Op::CopyMemory | spirv_headers::Op::CopyMemorySized => {
                    function.accessed.push(spv_words[word_offset + 2]);
                    function.accessed.push(spv_words[word_offset + 3]);
                }
                _ => {}
            }
        }

        function.callees.sort_by(|a, b| {
            let a_id = a.callee;
            let b_id = b.callee;
            a_id.cmp(&b_id)
        });
        function.callees.dedup();

        function.accessed.sort_by(|a, b| a.cmp(&b));
        function.accessed.dedup();

        Ok(function)
    }

    fn parse_functions(
        &mut self,
        spv_words: &[u32],
        _module: &mut super::ShaderModule,
    ) -> Result<(), String> {
        self.functions.reserve(self.function_count);
        for mut node_index in 0..self.nodes.len() {
            let current_node_index = node_index;
            let op = &self.nodes[current_node_index].op;
            if op != &spirv_headers::Op::Function {
                continue;
            }

            let mut function_definition = false;
            for sub_node_index in node_index..self.nodes.len() {
                node_index = sub_node_index; // TODO: Verify this is correct
                if self.nodes[node_index].op == spirv_headers::Op::Label {
                    function_definition = true;
                    break;
                }

                if self.nodes[node_index].op == spirv_headers::Op::FunctionEnd {
                    break;
                }
            }

            if !function_definition {
                continue;
            }

            let function = self.parse_function(&spv_words, current_node_index, node_index)?;
            self.functions.push(function);
        }

        self.functions.sort_by(|a, b| {
            let a_id = a.id as i32;
            let b_id = b.id as i32;
            a_id.cmp(&b_id)
        });

        // Link up callee pointers to optimize for traversal.
        for function_index in 0..self.functions.len() {
            if self.functions[function_index].callees.len() > 0 {
                let mut callee_function = 0;
                for callee_index in 0..self.functions[function_index].callees.len() {
                    let callee_id = self.functions[function_index].callees[callee_index].callee;
                    while self.functions[callee_function].id != callee_id {
                        callee_function += 1;
                        if callee_function >= self.function_count {
                            return Err("Invalid SPIR-V ID reference".into());
                        }
                    }
                    self.functions[function_index].callees[callee_index].function = callee_function;
                }
            }
        }

        Ok(())
    }

    fn parse_member_counts(
        &mut self,
        spv_words: &[u32],
        _: &mut super::ShaderModule,
    ) -> Result<(), String> {
        for node_index in 0..self.nodes.len() {
            let op = &self.nodes[node_index].op;
            if op != &spirv_headers::Op::MemberName && op != &spirv_headers::Op::MemberDecorate {
                continue;
            }

            let word_offset = self.nodes[node_index].word_offset as usize;
            let target_id = spv_words[word_offset + 1];
            let member_index = spv_words[word_offset + 2];

            // Not all nodes are parsed
            if let Some(target_node_index) = self.find_node(target_id) {
                let mut target_node = &mut self.nodes[target_node_index];
                target_node.member_count =
                    std::cmp::max(target_node.member_count, member_index + 1);
            }
        }

        for node in &mut self.nodes {
            let member_count = node.member_count as usize;
            if member_count == 0 {
                continue;
            }

            node.member_names.resize(member_count, String::new());
            node.member_decorations
                .resize(member_count, Decorations::default());
        }

        Ok(())
    }

    fn parse_names(
        &mut self,
        spv_words: &[u32],
        _module: &mut super::ShaderModule,
    ) -> Result<(), String> {
        for node_index in 0..self.nodes.len() {
            let node_op = self.nodes[node_index].op;
            if node_op != spirv_headers::Op::MemberName && node_op != spirv_headers::Op::Name {
                continue;
            }

            let word_offset = self.nodes[node_index].word_offset as usize;
            let target_id = spv_words[word_offset + 1];
            if let Some(target_node_index) = self.find_node(target_id) {
                let node_name = self.nodes[node_index].name.to_owned();
                let mut target_node = &mut self.nodes[target_node_index];
                if node_op == spirv_headers::Op::MemberName {
                    let member_index = spv_words[word_offset + 2] as usize;
                    target_node.member_names[member_index] = node_name;
                } else {
                    target_node.name = node_name;
                }
            }
        }

        Ok(())
    }

    fn parse_decorations(
        &mut self,
        spv_words: &[u32],
        _module: &mut super::ShaderModule,
    ) -> Result<(), String> {
        for node_index in 0..self.nodes.len() {
            let node_op = self.nodes[node_index].op;
            if node_op != spirv_headers::Op::Decorate
                && node_op != spirv_headers::Op::MemberDecorate
                && node_op != spirv_headers::Op::DecorateId
                && node_op != spirv_headers::Op::DecorateString
                && node_op != spirv_headers::Op::MemberDecorateStringGOOGLE
            {
                continue;
            }

            let word_offset = self.nodes[node_index].word_offset as usize;
            let member_offset = if node_op == spirv_headers::Op::MemberDecorate {
                1
            } else {
                0
            };

            if let Some(decoration) =
                spirv_headers::Decoration::from_u32(spv_words[word_offset + member_offset + 2])
            {
                let affects_reflection = match decoration {
                    spirv_headers::Decoration::Block
                    | spirv_headers::Decoration::BufferBlock
                    | spirv_headers::Decoration::ColMajor
                    | spirv_headers::Decoration::RowMajor
                    | spirv_headers::Decoration::ArrayStride
                    | spirv_headers::Decoration::MatrixStride
                    | spirv_headers::Decoration::BuiltIn
                    | spirv_headers::Decoration::NoPerspective
                    | spirv_headers::Decoration::Flat
                    | spirv_headers::Decoration::NonWritable
                    | spirv_headers::Decoration::Location
                    | spirv_headers::Decoration::Binding
                    | spirv_headers::Decoration::DescriptorSet
                    | spirv_headers::Decoration::Offset
                    | spirv_headers::Decoration::InputAttachmentIndex
                    | spirv_headers::Decoration::HlslCounterBufferGOOGLE
                    | spirv_headers::Decoration::HlslSemanticGOOGLE => true,
                    _ => false,
                };

                if !affects_reflection {
                    continue;
                }

                let target_id = spv_words[word_offset + 1];
                if let Some(target_node_index) = self.find_node(target_id) {
                    let target_node = &mut self.nodes[target_node_index];
                    let mut target_decorations = if node_op == spirv_headers::Op::MemberDecorate {
                        let member_index = spv_words[word_offset + 2] as usize;
                        &mut target_node.member_decorations[member_index]
                    } else {
                        &mut target_node.decorations
                    };

                    match decoration {
                        spirv_headers::Decoration::Block => {
                            target_decorations.is_block = true;
                        }
                        spirv_headers::Decoration::BufferBlock => {
                            target_decorations.is_buffer_block = true;
                        }
                        spirv_headers::Decoration::ColMajor => {
                            target_decorations.is_column_major = true;
                        }
                        spirv_headers::Decoration::RowMajor => {
                            target_decorations.is_row_major = true;
                        }
                        spirv_headers::Decoration::ArrayStride => {
                            let word_offset = word_offset + member_offset + 3;
                            target_decorations.array_stride = spv_words[word_offset];
                        }
                        spirv_headers::Decoration::MatrixStride => {
                            let word_offset = word_offset + member_offset + 3;
                            target_decorations.matrix_stride = spv_words[word_offset];
                        }
                        spirv_headers::Decoration::BuiltIn => {
                            let word_offset = word_offset + member_offset + 3;
                            target_decorations.built_in =
                                spirv_headers::BuiltIn::from_u32(spv_words[word_offset]);
                        }
                        spirv_headers::Decoration::NoPerspective => {
                            target_decorations.is_noperspective = true;
                        }
                        spirv_headers::Decoration::Flat => {
                            target_decorations.is_flat = true;
                        }
                        spirv_headers::Decoration::NonWritable => {
                            target_decorations.is_non_writable = true;
                        }
                        spirv_headers::Decoration::Location => {
                            let word_offset = word_offset + member_offset + 3;
                            target_decorations.location.value = spv_words[word_offset];
                            target_decorations.location.word_offset = word_offset as u32;
                        }
                        spirv_headers::Decoration::Binding => {
                            let word_offset = word_offset + member_offset + 3;
                            target_decorations.binding.value = spv_words[word_offset];
                            target_decorations.binding.word_offset = word_offset as u32;
                        }
                        spirv_headers::Decoration::DescriptorSet => {
                            let word_offset = word_offset + member_offset + 3;
                            target_decorations.set.value = spv_words[word_offset];
                            target_decorations.set.word_offset = word_offset as u32;
                        }
                        spirv_headers::Decoration::Offset => {
                            let word_offset = word_offset + member_offset + 3;
                            target_decorations.offset.value = spv_words[word_offset];
                            target_decorations.offset.word_offset = word_offset as u32;
                        }
                        spirv_headers::Decoration::InputAttachmentIndex => {
                            let word_offset = word_offset + member_offset + 3;
                            target_decorations.input_attachment_index.value =
                                spv_words[word_offset];
                            target_decorations.input_attachment_index.word_offset =
                                word_offset as u32;
                        }
                        spirv_headers::Decoration::HlslCounterBufferGOOGLE => {
                            let word_offset = word_offset + member_offset + 3;
                            target_decorations.uav_counter_buffer.value = spv_words[word_offset];
                            target_decorations.uav_counter_buffer.word_offset = word_offset as u32;
                        }
                        spirv_headers::Decoration::HlslSemanticGOOGLE => {
                            let word_offset = word_offset + member_offset + 3;

                            target_decorations.semantic.value = unsafe {
                                let semantic_ptr = spv_words
                                    .as_ptr()
                                    .offset((word_offset / SPIRV_WORD_SIZE) as isize)
                                    as *const _;
                                CStr::from_ptr(semantic_ptr).to_string_lossy().into_owned()
                            };

                            target_decorations.semantic.word_offset = word_offset as u32;
                        }
                        _ => {}
                    }
                } else {
                    return Err("Invalid SPIR-V ID reference".into());
                }
            } else {
                return Err("Invalid SPIR-V decoration".into());
            }
        }

        Ok(())
    }

    fn apply_decorations(
        decorations: &Decorations,
    ) -> Result<crate::types::ReflectDecorationFlags, String> {
        let mut flags = crate::types::ReflectDecorationFlags::NONE;

        if decorations.is_block {
            flags |= crate::types::ReflectDecorationFlags::BLOCK;
        }

        if decorations.is_buffer_block {
            flags |= crate::types::ReflectDecorationFlags::BUFFER_BLOCK;
        }

        if decorations.is_row_major {
            flags |= crate::types::ReflectDecorationFlags::ROW_MAJOR;
        }

        if decorations.is_column_major {
            flags |= crate::types::ReflectDecorationFlags::COLUMN_MAJOR;
        }

        if decorations.built_in.is_some() {
            flags |= crate::types::ReflectDecorationFlags::BUILT_IN;
        }

        if decorations.is_noperspective {
            flags |= crate::types::ReflectDecorationFlags::NO_PERSPECTIVE;
        }

        if decorations.is_flat {
            flags |= crate::types::ReflectDecorationFlags::FLAT;
        }

        if decorations.is_non_writable {
            flags |= crate::types::ReflectDecorationFlags::NON_WRITABLE;
        }

        Ok(flags)
    }

    fn parse_type(
        &mut self,
        spv_words: &[u32],
        module: &mut super::ShaderModule,
        node_index: usize,
        struct_member_decorations: Option<(/* node */ usize, /* member */ usize)>,
        type_description: &mut crate::types::ReflectTypeDescription,
    ) -> Result<(), String> {
        let word_offset = self.nodes[node_index].word_offset as usize;
        type_description.members.resize(
            self.nodes[node_index].member_count as usize,
            crate::types::ReflectTypeDescription::default(),
        );

        if type_description.id == std::u32::MAX {
            type_description.id = self.nodes[node_index].result_id;
            type_description.op = crate::types::ReflectOp(self.nodes[node_index].op);
            type_description.decoration_flags = crate::types::ReflectDecorationFlags::NONE;
        }

        type_description.decoration_flags |=
            Self::apply_decorations(&self.nodes[node_index].decorations)?;

        match self.nodes[node_index].op {
            spirv_headers::Op::TypeOpaque => {}
            spirv_headers::Op::TypeVoid => {
                type_description.type_flags |= crate::types::ReflectTypeFlags::VOID
            }
            spirv_headers::Op::TypeBool => {
                type_description.type_flags |= crate::types::ReflectTypeFlags::BOOL
            }
            spirv_headers::Op::TypeSampler => {
                type_description.type_flags |= crate::types::ReflectTypeFlags::EXTERNAL_SAMPLER
            }
            spirv_headers::Op::TypeInt => {
                type_description.type_flags |= crate::types::ReflectTypeFlags::INT;
                type_description.traits.numeric.scalar.width = spv_words[word_offset + 2];
                type_description.traits.numeric.scalar.signedness = spv_words[word_offset + 3];
            }
            spirv_headers::Op::TypeFloat => {
                type_description.type_flags |= crate::types::ReflectTypeFlags::FLOAT;
                type_description.traits.numeric.scalar.width = spv_words[word_offset + 2];
            }
            spirv_headers::Op::TypeVector => {
                type_description.type_flags |= crate::types::ReflectTypeFlags::VECTOR;
                let component_type_id = spv_words[word_offset + 2];
                type_description.traits.numeric.vector.component_count = spv_words[word_offset + 3];
                if let Some(next_node_index) = self.find_node(component_type_id) {
                    self.parse_type(&spv_words, module, next_node_index, None, type_description)?;
                } else {
                    return Err("Invalid SPIR-V ID reference".into());
                }
            }
            spirv_headers::Op::TypeMatrix => {
                type_description.type_flags |= crate::types::ReflectTypeFlags::MATRIX;
                let column_type_id = spv_words[word_offset + 2];
                type_description.traits.numeric.matrix.column_count = spv_words[word_offset + 3];
                if let Some(next_node_index) = self.find_node(column_type_id) {
                    self.parse_type(&spv_words, module, next_node_index, None, type_description)?;
                } else {
                    return Err("Invalid SPIR-V ID reference".into());
                }
                type_description.traits.numeric.matrix.row_count =
                    type_description.traits.numeric.vector.component_count;
                if let Some(ref struct_member_index) = struct_member_decorations {
                    let member_node = &self.nodes[struct_member_index.0];
                    let member_decorations = &member_node.member_decorations[struct_member_index.1];
                    type_description.traits.numeric.matrix.stride =
                        member_decorations.matrix_stride;
                } else {
                    type_description.traits.numeric.matrix.stride =
                        self.nodes[node_index].decorations.matrix_stride;
                }
            }
            spirv_headers::Op::TypeImage => {
                type_description.type_flags |= crate::types::ReflectTypeFlags::EXTERNAL_IMAGE;
                type_description.traits.image.dim =
                    spirv_headers::Dim::from_u32(spv_words[word_offset + 3]).into();
                type_description.traits.image.depth = spv_words[word_offset + 4];
                type_description.traits.image.arrayed = spv_words[word_offset + 5];
                type_description.traits.image.ms = spv_words[word_offset + 6];
                type_description.traits.image.sampled = spv_words[word_offset + 7];
                type_description.traits.image.image_format =
                    spirv_headers::ImageFormat::from_u32(spv_words[word_offset + 8]).into();
            }
            spirv_headers::Op::TypeSampledImage => {
                type_description.type_flags |=
                    crate::types::ReflectTypeFlags::EXTERNAL_SAMPLED_IMAGE;
                let image_type_id = spv_words[word_offset + 2];
                if let Some(next_node_index) = self.find_node(image_type_id) {
                    self.parse_type(&spv_words, module, next_node_index, None, type_description)?;
                } else {
                    return Err("Invalid SPIR-V ID reference".into());
                }
            }
            spirv_headers::Op::TypeArray => {
                type_description.type_flags |= crate::types::ReflectTypeFlags::ARRAY;
                let element_type_id = spv_words[word_offset + 2];
                let length_id = spv_words[word_offset + 3];
                type_description.traits.array.stride =
                    self.nodes[node_index].decorations.array_stride;
                if let Some(length_node_index) = self.find_node(length_id) {
                    let length = spv_words[self.nodes[length_node_index].word_offset as usize + 3];
                    type_description.traits.array.dims.push(length);
                    if let Some(next_node_index) = self.find_node(element_type_id) {
                        self.parse_type(
                            &spv_words,
                            module,
                            next_node_index,
                            None,
                            type_description,
                        )?;
                    } else {
                        return Err("Invalid SPIR-V ID reference".into());
                    }
                } else {
                    return Err("Invalid SPIR-V ID reference".into());
                }
            }
            spirv_headers::Op::TypeRuntimeArray => {
                let element_type_id = spv_words[word_offset + 2];
                if let Some(next_node_index) = self.find_node(element_type_id) {
                    self.parse_type(&spv_words, module, next_node_index, None, type_description)?;
                } else {
                    return Err("Invalid SPIR-V ID reference".into());
                }
            }
            spirv_headers::Op::TypeStruct => {
                type_description.type_flags |= crate::types::ReflectTypeFlags::STRUCT
                    | crate::types::ReflectTypeFlags::EXTERNAL_BLOCK;
                let mut member_index = 0;
                for word_index in 2..self.nodes[node_index].word_count as usize {
                    let member_id = spv_words[word_offset + word_index];
                    if let Some(member_node_index) = self.find_node(member_id) {
                        assert!(member_index < type_description.members.len());
                        let mut member_type_description =
                            &mut type_description.members[member_index];
                        member_type_description.id = member_id;
                        member_type_description.op =
                            crate::types::ReflectOp(self.nodes[member_node_index].op);
                        self.parse_type(
                            &spv_words,
                            module,
                            member_node_index,
                            Some((node_index, member_index)),
                            &mut member_type_description,
                        )?;
                        member_type_description.struct_member_name =
                            self.nodes[node_index].member_names[member_index].to_owned();
                    } else {
                        return Err("Invalid SPIR-V ID reference".into());
                    }

                    member_index += 1;
                }
            }
            spirv_headers::Op::TypePointer => {
                type_description.storage_class =
                    spirv_headers::StorageClass::from_u32(spv_words[word_offset + 2]).into();
                if type_description.storage_class == crate::types::ReflectStorageClass::Undefined {
                    return Err("Invalid SPIR-V ID reference".into());
                }
                let type_id = spv_words[word_offset + 3];
                if let Some(next_node_index) = self.find_node(type_id) {
                    self.parse_type(&spv_words, module, next_node_index, None, type_description)?;
                } else {
                    return Err("Invalid SPIR-V ID reference".into());
                }
            }
            _ => {}
        }

        if type_description.type_name.is_empty() {
            type_description.type_name = self.nodes[node_index].name.to_owned();
        }

        Ok(())
    }

    fn parse_types(
        &mut self,
        spv_words: &[u32],
        module: &mut super::ShaderModule,
    ) -> Result<(), String> {
        module.internal.type_descriptions.reserve(self.type_count);
        for node_index in 0..self.nodes.len() {
            if !self.nodes[node_index].is_type {
                continue;
            }
            let mut type_description = crate::types::ReflectTypeDescription::default();
            self.parse_type(&spv_words, module, node_index, None, &mut type_description)?;
            module.internal.type_descriptions.push(type_description);
        }
        Ok(())
    }

    fn parse_descriptor_bindings(
        &mut self,
        _spv_words: &[u32],
        module: &mut super::ShaderModule,
    ) -> Result<(), String> {
        let mut binding_nodes = Vec::with_capacity(16);

        for node_index in 0..self.nodes.len() {
            let node = &self.nodes[node_index];
            if node.op != spirv_headers::Op::Variable
            /*|| (node.storage_class != spirv_headers::StorageClass::Uniform
            && node.storage_class != spirv_headers::StorageClass::UniformConstant)*/
            {
                continue;
            }

            if node.decorations.set.value == std::u32::MAX
                || node.decorations.binding.value == std::u32::MAX
            {
                continue;
            }

            binding_nodes.push(node_index);
        }

        if binding_nodes.len() > 0 {
            module
                .internal
                .descriptor_bindings
                .reserve(binding_nodes.len());
            for node_index in binding_nodes {
                let mut descriptor_type = crate::types::ReflectDescriptorType::Undefined;

                if let Some(type_index) = module.internal.find_type(self.nodes[node_index].type_id)
                {
                    // Resolve pointer types
                    let resolved_type_index = if *module.internal.type_descriptions[type_index].op
                        == spirv_headers::Op::TypePointer
                    {
                        match module.internal.type_descriptions[type_index].storage_class {
                            types::ReflectStorageClass::Uniform
                            | types::ReflectStorageClass::UniformConstant => {
                                descriptor_type =
                                    crate::types::ReflectDescriptorType::UniformBuffer;
                            }
                            types::ReflectStorageClass::StorageBuffer => {
                                descriptor_type =
                                    crate::types::ReflectDescriptorType::StorageBuffer;
                            }
                            _ => todo!(
                                "{:?}",
                                module.internal.type_descriptions[type_index].storage_class
                            ),
                        }

                        if let Some(type_node_index) =
                            self.find_node(module.internal.type_descriptions[type_index].id)
                        {
                            let type_node = &self.nodes[type_node_index];
                            if let Some(pointer_type_index) =
                                module.internal.find_type(type_node.type_id)
                            {
                                pointer_type_index
                            } else {
                                return Err("Invalid SPIR-V ID reference".into());
                            }
                        } else {
                            return Err("Invalid SPIR-V ID reference".into());
                        }
                    } else {
                        type_index
                    };

                    let type_description = &module.internal.type_descriptions[resolved_type_index];

                    let mut count = 1;
                    for dim_index in 0..type_description.traits.array.dims.len() {
                        count *= type_description.traits.array.dims[dim_index];
                    }

                    let is_sampled_image = (type_description.type_flags
                        & crate::types::ReflectTypeFlags::SAMPLED_MASK)
                        == crate::types::ReflectTypeFlags::SAMPLED_MASK;
                    let is_external_image = (type_description.type_flags
                        & crate::types::ReflectTypeFlags::EXTERNAL_MASK)
                        == crate::types::ReflectTypeFlags::EXTERNAL_IMAGE;

                    let node = &self.nodes[node_index];
                    module.internal.descriptor_bindings.push(
                        crate::types::ReflectDescriptorBinding {
                            spirv_id: node.result_id,
                            word_offset: (
                                node.decorations.binding.word_offset,
                                node.decorations.set.word_offset,
                            ),
                            name: node.name.to_owned(),
                            descriptor_type,
                            binding: node.decorations.binding.value,
                            input_attachment_index: node.decorations.input_attachment_index.value,
                            set: node.decorations.set.value,
                            count,
                            accessed: false,
                            uav_counter_id: node.decorations.uav_counter_buffer.value,
                            uav_counter_index: std::usize::MAX,
                            type_index: Some(resolved_type_index),
                            resource_type: crate::types::ReflectResourceTypeFlags::UNDEFINED,
                            block: crate::types::ReflectBlockVariable::default(),
                            image: if is_external_image || is_sampled_image {
                                type_description.traits.image.clone()
                            } else {
                                crate::types::ReflectImageTraits::default()
                            },
                            array: crate::types::ReflectBindingArrayTraits {
                                dims: type_description.traits.array.dims.clone(),
                            },
                        },
                    );
                } else {
                    return Err("Invalid SPIR-V ID reference".into());
                }
            }

            module.internal.descriptor_bindings.sort_by(|a, b| {
                let a_binding = a.binding as i32;
                let b_binding = b.binding as i32;
                let a_spirv_id = a.spirv_id as i32;
                let b_spirv_id = b.spirv_id as i32;
                Ord::cmp(&a_binding, &b_binding).then(Ord::cmp(&a_spirv_id, &b_spirv_id))
            });
        }

        Ok(())
    }

    fn parse_descriptor_type(&mut self, module: &mut super::ShaderModule) -> Result<(), String> {
        const SAMPLED_IMAGE: u32 = 1;
        const STORAGE_IMAGE: u32 = 2;
        for binding_index in 0..module.internal.descriptor_bindings.len() {
            let mut descriptor_binding = &mut module.internal.descriptor_bindings[binding_index];
            if let Some(type_index) = descriptor_binding.type_index {
                let type_description = &module.internal.type_descriptions[type_index];
                match type_description.type_flags & crate::types::ReflectTypeFlags::EXTERNAL_MASK {
                    crate::types::ReflectTypeFlags::EXTERNAL_BLOCK => {
                        // The descriptor type may have been already found when
                        // processing Op::TypePointer.
                        if crate::types::ReflectDescriptorType::Undefined
                            == descriptor_binding.descriptor_type
                        {
                            if type_description
                                .decoration_flags
                                .contains(crate::types::ReflectDecorationFlags::BLOCK)
                            {
                                descriptor_binding.descriptor_type =
                                    match type_description.storage_class {
                                        crate::types::ReflectStorageClass::StorageBuffer => {
                                            crate::types::ReflectDescriptorType::StorageBuffer
                                        }

                                        crate::types::ReflectStorageClass::Uniform => {
                                            crate::types::ReflectDescriptorType::UniformBuffer
                                        }

                                        _ => todo!(
                                            "{:?} in {:#?}",
                                            type_description.storage_class,
                                            type_description
                                        ),
                                    }
                            } else if type_description
                                .decoration_flags
                                .contains(crate::types::ReflectDecorationFlags::BUFFER_BLOCK)
                            {
                                descriptor_binding.descriptor_type =
                                    crate::types::ReflectDescriptorType::StorageBuffer;
                            } else {
                                return Err("Invalid SPIR-V struct type".into());
                            }
                        }
                    }
                    crate::types::ReflectTypeFlags::EXTERNAL_IMAGE => {
                        if descriptor_binding.image.dim == crate::types::ReflectDimension::Buffer {
                            if descriptor_binding.image.sampled == SAMPLED_IMAGE {
                                descriptor_binding.descriptor_type =
                                    crate::types::ReflectDescriptorType::UniformTexelBuffer;
                            } else if descriptor_binding.image.sampled == STORAGE_IMAGE {
                                descriptor_binding.descriptor_type =
                                    crate::types::ReflectDescriptorType::StorageTexelBuffer;
                            } else {
                                return Err("Invalid SPIR-V texel buffer sampling".into());
                            }
                        } else if descriptor_binding.image.dim
                            == crate::types::ReflectDimension::SubPassData
                        {
                            descriptor_binding.descriptor_type =
                                crate::types::ReflectDescriptorType::InputAttachment;
                        } else {
                            if descriptor_binding.image.sampled == SAMPLED_IMAGE {
                                descriptor_binding.descriptor_type =
                                    crate::types::ReflectDescriptorType::SampledImage;
                            } else if descriptor_binding.image.sampled == STORAGE_IMAGE {
                                descriptor_binding.descriptor_type =
                                    crate::types::ReflectDescriptorType::StorageImage;
                            } else {
                                return Err("Invalid SPIR-V image sampling".into());
                            }
                        }
                    }
                    crate::types::ReflectTypeFlags::EXTERNAL_SAMPLER => {
                        descriptor_binding.descriptor_type =
                            crate::types::ReflectDescriptorType::Sampler;
                    }
                    crate::types::ReflectTypeFlags::SAMPLED_MASK => {
                        if descriptor_binding.image.dim == crate::types::ReflectDimension::Buffer {
                            if descriptor_binding.image.sampled == SAMPLED_IMAGE {
                                descriptor_binding.descriptor_type =
                                    crate::types::ReflectDescriptorType::UniformBuffer;
                            } else if descriptor_binding.image.sampled == STORAGE_IMAGE {
                                descriptor_binding.descriptor_type =
                                    crate::types::ReflectDescriptorType::StorageBuffer;
                            } else {
                                return Err("Invalid SPIR-V texel buffer sampling".into());
                            }
                        } else {
                            descriptor_binding.descriptor_type =
                                crate::types::ReflectDescriptorType::CombinedImageSampler;
                        }
                    }
                    _ => {
                        return Err("Invalid SPIR-V type flag".into());
                    }
                }

                descriptor_binding.resource_type = match descriptor_binding.descriptor_type {
                    crate::types::ReflectDescriptorType::Sampler => {
                        crate::types::ReflectResourceTypeFlags::SAMPLER
                    }
                    crate::types::ReflectDescriptorType::CombinedImageSampler => {
                        crate::types::ReflectResourceTypeFlags::SAMPLER
                            | crate::types::ReflectResourceTypeFlags::SHADER_RESOURCE_VIEW
                    }
                    crate::types::ReflectDescriptorType::SampledImage
                    | crate::types::ReflectDescriptorType::UniformTexelBuffer => {
                        crate::types::ReflectResourceTypeFlags::SHADER_RESOURCE_VIEW
                    }
                    crate::types::ReflectDescriptorType::StorageImage
                    | crate::types::ReflectDescriptorType::StorageTexelBuffer
                    | crate::types::ReflectDescriptorType::StorageBuffer
                    | crate::types::ReflectDescriptorType::StorageBufferDynamic => {
                        crate::types::ReflectResourceTypeFlags::UNORDERED_ACCESS_VIEW
                    }
                    crate::types::ReflectDescriptorType::UniformBuffer
                    | crate::types::ReflectDescriptorType::UniformBufferDynamic => {
                        crate::types::ReflectResourceTypeFlags::CONSTANT_BUFFER_VIEW
                    }
                    _ => crate::types::ReflectResourceTypeFlags::UNDEFINED,
                };
            } else {
                return Err("Invalid SPIR-V type description".into());
            }
        }
        Ok(())
    }

    fn parse_counter_bindings(
        &mut self,
        _spv_words: &[u32],
        module: &mut super::ShaderModule,
    ) -> Result<(), String> {
        for descriptor_binding_index in 0..module.internal.descriptor_bindings.len() {
            let descriptor_binding = &module.internal.descriptor_bindings[descriptor_binding_index];
            if descriptor_binding.descriptor_type
                != crate::types::ReflectDescriptorType::StorageBuffer
            {
                continue;
            }

            let mut counter_binding_index = std::usize::MAX;

            if descriptor_binding.uav_counter_id != std::u32::MAX {
                // Modern approach.
                for counter_descriptor_binding_index in 0..module.internal.descriptor_bindings.len()
                {
                    let counter_descriptor_binding =
                        &module.internal.descriptor_bindings[counter_descriptor_binding_index];
                    if counter_descriptor_binding.descriptor_type
                        != crate::types::ReflectDescriptorType::StorageBuffer
                    {
                        continue;
                    }

                    if descriptor_binding.uav_counter_id == counter_descriptor_binding.spirv_id {
                        counter_binding_index = counter_descriptor_binding_index;
                        break;
                    }
                }
            } else {
                // Legacy approach.
                let counter_name = format!("{}@count", &descriptor_binding.name);

                for counter_descriptor_binding_index in 0..module.internal.descriptor_bindings.len()
                {
                    let counter_descriptor_binding =
                        &module.internal.descriptor_bindings[counter_descriptor_binding_index];
                    if counter_descriptor_binding.descriptor_type
                        != crate::types::ReflectDescriptorType::StorageBuffer
                    {
                        continue;
                    }

                    if counter_descriptor_binding.name == counter_name {
                        counter_binding_index = counter_descriptor_binding_index;
                        break;
                    }
                }
            }

            module.internal.descriptor_bindings[descriptor_binding_index].uav_counter_index =
                counter_binding_index;
        }

        Ok(())
    }

    fn parse_descriptor_block_variable(
        &mut self,
        module: &super::ShaderModule,
        type_description: &crate::types::ReflectTypeDescription,
        variable: &mut crate::types::ReflectBlockVariable,
    ) -> Result<(), String> {
        let mut has_no_write = false;
        let resolved_type_description = if type_description.members.len() > 0 {
            if let Some(type_node_index) = self.find_node(type_description.id) {
                variable.members.reserve(type_description.members.len());

                let (resolved_node_index, resolved_type) = match self.nodes[type_node_index].op {
                    spirv_headers::Op::TypeArray => {
                        let mut resolved_node_index = type_node_index;
                        while self.nodes[type_node_index].op == spirv_headers::Op::TypeArray {
                            let element_type_id =
                                self.nodes[type_node_index].array_traits.element_type_id;
                            if let Some(test_type_node_index) = self.find_node(element_type_id) {
                                resolved_node_index = test_type_node_index;
                            } else {
                                return Err("Invalid SPIR-V ID reference".into());
                            }
                        }
                        (resolved_node_index, type_description)
                    }
                    spirv_headers::Op::TypeRuntimeArray => {
                        if let Some(resolved_type_index) = module
                            .internal
                            .find_type(self.nodes[type_node_index].array_traits.element_type_id)
                        {
                            let resolved_type_description =
                                &module.internal.type_descriptions[resolved_type_index];
                            if let Some(resolved_type_node_index) =
                                self.find_node(resolved_type_description.id)
                            {
                                (resolved_type_node_index, resolved_type_description)
                            } else {
                                return Err("Invalid SPIR-V ID reference".into());
                            }
                        } else {
                            return Err("Invalid SPIR-V ID reference".into());
                        }
                    }
                    _ => (type_node_index, type_description),
                };

                for member_index in 0..type_description.members.len() {
                    let mut member_variable = crate::types::ReflectBlockVariable::default();

                    let member_type_description = &type_description.members[member_index];
                    if (member_type_description.type_flags & crate::types::ReflectTypeFlags::STRUCT)
                        == crate::types::ReflectTypeFlags::STRUCT
                    {
                        self.parse_descriptor_block_variable(
                            module,
                            member_type_description,
                            &mut member_variable,
                        )?;
                    }

                    let type_node = &self.nodes[resolved_node_index];

                    member_variable.name = type_node.member_names[member_index].to_owned();
                    member_variable.offset =
                        type_node.member_decorations[member_index].offset.value;
                    member_variable.decoration_flags =
                        Self::apply_decorations(&type_node.member_decorations[member_index])?;

                    if member_variable
                        .decoration_flags
                        .contains(crate::types::ReflectDecorationFlags::NON_WRITABLE)
                    {
                        has_no_write = true;
                    }

                    if *type_description.op == spirv_headers::Op::TypeArray {
                        member_variable.array = member_type_description.traits.array.clone();
                    }

                    member_variable.numeric = member_type_description.traits.numeric.clone();
                    member_variable.type_description = member_type_description.to_owned();
                    variable.members.push(member_variable);
                }

                resolved_type
            } else {
                return Err("Invalid SPIR-V ID reference".into());
            }
        } else {
            type_description
        };

        if has_no_write {
            variable.decoration_flags |= crate::types::ReflectDecorationFlags::NON_WRITABLE;
        }

        let var_type_description = resolved_type_description.to_owned();
        variable.name = var_type_description.type_name.to_owned();
        variable.type_description = var_type_description;
        Ok(())
    }

    fn parse_descriptor_block_variable_sizes(
        &mut self,
        //_spv_words: &[u32],
        module: &mut super::ShaderModule,
        is_parent_root: bool,
        is_parent_aos: bool,
        is_parent_rta: bool,
        variable: &mut crate::types::ReflectBlockVariable,
    ) -> Result<(), String> {
        if variable.members.len() > 0 {
            // Calculate absolute offsets
            for mut variable_member in &mut variable.members {
                variable_member.absolute_offset = if is_parent_root {
                    variable_member.offset
                } else {
                    if is_parent_aos {
                        0
                    } else {
                        variable_member.offset + variable.absolute_offset
                    }
                };
            }

            // Calculate size
            for mut variable_member in &mut variable.members {
                match *variable_member.type_description.op {
                    spirv_headers::Op::TypeBool => {
                        variable_member.size = SPIRV_WORD_SIZE as u32;
                    }
                    spirv_headers::Op::TypeInt | spirv_headers::Op::TypeFloat => {
                        variable_member.size =
                            variable_member.type_description.traits.numeric.scalar.width
                                / SPIRV_BYTE_WIDTH as u32;
                    }
                    spirv_headers::Op::TypeVector => {
                        variable_member.size = variable_member
                            .type_description
                            .traits
                            .numeric
                            .vector
                            .component_count
                            * (variable_member.type_description.traits.numeric.scalar.width
                                / SPIRV_BYTE_WIDTH as u32);
                    }
                    spirv_headers::Op::TypeMatrix => {
                        if variable_member
                            .decoration_flags
                            .contains(crate::types::ReflectDecorationFlags::COLUMN_MAJOR)
                        {
                            variable_member.size = variable_member
                                .type_description
                                .traits
                                .numeric
                                .matrix
                                .column_count
                                * variable_member.numeric.matrix.stride;
                        } else if variable_member
                            .decoration_flags
                            .contains(crate::types::ReflectDecorationFlags::ROW_MAJOR)
                        {
                            variable_member.size = variable_member
                                .type_description
                                .traits
                                .numeric
                                .matrix
                                .row_count
                                * variable_member.numeric.matrix.stride;
                        }
                    }
                    spirv_headers::Op::TypeArray => {
                        if (variable_member.type_description.type_flags
                            & crate::types::ReflectTypeFlags::STRUCT)
                            == crate::types::ReflectTypeFlags::STRUCT
                        {
                            // Struct of structs
                            self.parse_descriptor_block_variable_sizes(
                                module,
                                false,
                                true,
                                is_parent_rta,
                                &mut variable_member,
                            )?;
                        }
                        variable_member.size = if variable_member.array.dims.len() > 0 {
                            let mut element_count = 1;
                            for dim in &variable_member.array.dims {
                                element_count *= dim;
                            }
                            element_count
                        } else {
                            0
                        } * variable_member.array.stride;
                    }
                    spirv_headers::Op::TypeRuntimeArray => {
                        if (variable_member.type_description.type_flags
                            & crate::types::ReflectTypeFlags::STRUCT)
                            == crate::types::ReflectTypeFlags::STRUCT
                        {
                            self.parse_descriptor_block_variable_sizes(
                                module,
                                false,
                                true,
                                true,
                                &mut variable_member,
                            )?;
                        }
                    }
                    spirv_headers::Op::TypeStruct => {
                        self.parse_descriptor_block_variable_sizes(
                            module,
                            false,
                            is_parent_aos,
                            is_parent_rta,
                            &mut variable_member,
                        )?;
                    }
                    _ => {}
                }
            }

            // Calculate padding
            for member_index in 0..(variable.members.len() - 1) {
                let next_member_index = member_index + 1;
                variable.members[member_index].padded_size = variable.members[next_member_index]
                    .offset
                    - variable.members[member_index].offset;
                if variable.members[member_index].size > variable.members[member_index].padded_size
                {
                    variable.members[member_index].size =
                        variable.members[member_index].padded_size;
                }
                if is_parent_rta {
                    variable.members[member_index].padded_size =
                        variable.members[member_index].size;
                }
            }

            // Round up last member to next multiple of SPIR-V data alignment
            const SPIRV_DATA_ALIGN: u32 = 16;
            let last_size = {
                let last_member_index = variable.members.len() - 1;
                let mut last_member_variable = &mut variable.members[last_member_index];

                last_member_variable.padded_size =
                    ((last_member_variable.offset + last_member_variable.size + SPIRV_DATA_ALIGN
                        - 1)
                        & !(SPIRV_DATA_ALIGN - 1))
                        - last_member_variable.offset;

                if last_member_variable.size > last_member_variable.padded_size {
                    last_member_variable.size = last_member_variable.padded_size;
                }

                if is_parent_rta {
                    last_member_variable.padded_size = last_member_variable.size;
                }

                last_member_variable.offset + last_member_variable.padded_size
            };

            variable.size = last_size;
            variable.padded_size = last_size;
        }

        Ok(())
    }

    fn parse_descriptor_blocks(
        &mut self,
        _spv_words: &[u32],
        module: &mut super::ShaderModule,
    ) -> Result<(), String> {
        for descriptor_binding_index in 0..module.internal.descriptor_bindings.len() {
            let descriptor_type =
                module.internal.descriptor_bindings[descriptor_binding_index].descriptor_type;
            if descriptor_type != crate::types::ReflectDescriptorType::UniformBuffer
                && descriptor_type != crate::types::ReflectDescriptorType::StorageBuffer
            {
                continue;
            }

            if let Some(type_index) =
                module.internal.descriptor_bindings[descriptor_binding_index].type_index
            {
                let mut block = module.internal.descriptor_bindings[descriptor_binding_index]
                    .block
                    .to_owned();

                let type_description = &module.internal.type_descriptions[type_index];
                self.parse_descriptor_block_variable(module, type_description, &mut block)?;

                // Top level uses descriptor name
                block.name = module.internal.descriptor_bindings[descriptor_binding_index]
                    .name
                    .to_owned();

                let is_parent_rta =
                    descriptor_type == crate::types::ReflectDescriptorType::StorageBuffer;
                self.parse_descriptor_block_variable_sizes(
                    module,
                    true,
                    false,
                    is_parent_rta,
                    &mut block,
                )?;
                if is_parent_rta {
                    block.size = 0;
                    block.padded_size = 0;
                }

                module.internal.descriptor_bindings[descriptor_binding_index].block = block;
            } else {
                return Err("Invalid SPIR-V type description".into());
            }
        }

        Ok(())
    }

    fn parse_push_constant_blocks(
        &mut self,
        _spv_words: &[u32],
        module: &mut super::ShaderModule,
    ) -> Result<(), String> {
        let mut block_count = 0;
        for node_index in 0..self.nodes.len() {
            let node = &self.nodes[node_index];
            if node.op != spirv_headers::Op::Variable
                || node.storage_class != spirv_headers::StorageClass::PushConstant
            {
                continue;
            }

            block_count += 1;
        }

        if block_count > 0 {
            module.internal.push_constant_blocks.reserve(block_count);

            for node_index in 0..self.nodes.len() {
                let node = &self.nodes[node_index];
                if node.op != spirv_headers::Op::Variable
                    || node.storage_class != spirv_headers::StorageClass::PushConstant
                {
                    continue;
                }

                if let Some(type_index) = module.internal.find_type(node.type_id) {
                    // Resolve pointer types
                    let resolved_type_index = if *module.internal.type_descriptions[type_index].op
                        == spirv_headers::Op::TypePointer
                    {
                        if let Some(type_node_index) =
                            self.find_node(module.internal.type_descriptions[type_index].id)
                        {
                            let type_node = &self.nodes[type_node_index];
                            if let Some(pointer_type_index) =
                                module.internal.find_type(type_node.type_id)
                            {
                                pointer_type_index
                            } else {
                                return Err("Invalid SPIR-V ID reference".into());
                            }
                        } else {
                            return Err("Invalid SPIR-V ID reference".into());
                        }
                    } else {
                        type_index
                    };

                    if let Some(_) =
                        self.find_node(module.internal.type_descriptions[resolved_type_index].id)
                    {
                        let mut push_constant = crate::types::ReflectBlockVariable::default();
                        let type_description =
                            &module.internal.type_descriptions[resolved_type_index];
                        self.parse_descriptor_block_variable(
                            module,
                            type_description,
                            &mut push_constant,
                        )?;
                        self.parse_descriptor_block_variable_sizes(
                            module,
                            true,
                            false,
                            false,
                            &mut push_constant,
                        )?;
                        module.internal.push_constant_blocks.push(push_constant);
                    } else {
                        return Err("Invalid SPIR-V ID reference".into());
                    }
                }
            }
        }

        Ok(())
    }

    fn parse_format(
        type_description: &crate::types::ReflectTypeDescription,
    ) -> Result<crate::types::ReflectFormat, String> {
        let is_signed = type_description.traits.numeric.scalar.signedness > 0;
        let is_int_type = type_description
            .type_flags
            .contains(crate::types::ReflectTypeFlags::INT)
            | type_description
                .type_flags
                .contains(crate::types::ReflectTypeFlags::BOOL);
        if type_description
            .type_flags
            .contains(crate::types::ReflectTypeFlags::VECTOR)
        {
            let component_count = type_description.traits.numeric.vector.component_count;
            if type_description
                .type_flags
                .contains(crate::types::ReflectTypeFlags::FLOAT)
            {
                match component_count {
                    4 => {
                        return Ok(crate::types::ReflectFormat::R32G32B32A32_SFLOAT);
                    }
                    3 => {
                        return Ok(crate::types::ReflectFormat::R32G32B32_SFLOAT);
                    }
                    2 => {
                        return Ok(crate::types::ReflectFormat::R32G32_SFLOAT);
                    }
                    _ => {}
                }
            } else if is_int_type {
                match component_count {
                    4 => {
                        return Ok(crate::types::ReflectFormat::R32G32B32A32_UINT);
                    }
                    3 => {
                        return Ok(crate::types::ReflectFormat::R32G32B32_UINT);
                    }
                    2 => {
                        return Ok(crate::types::ReflectFormat::R32G32_UINT);
                    }
                    _ => {}
                }
            }
        } else if type_description
            .type_flags
            .contains(crate::types::ReflectTypeFlags::FLOAT)
        {
            return Ok(crate::types::ReflectFormat::R32_SFLOAT);
        } else if is_int_type {
            if is_signed {
                return Ok(crate::types::ReflectFormat::R32_SINT);
            } else {
                return Ok(crate::types::ReflectFormat::R32_UINT);
            }
        } else if type_description
            .type_flags
            .contains(crate::types::ReflectTypeFlags::STRUCT)
        {
            return Ok(crate::types::ReflectFormat::Undefined);
        }

        Err(format!("Invalid type format: {:#?}", type_description))
    }

    fn parse_interface_variable(
        &self,
        module: &super::ShaderModule,
        built_in: &mut bool,
        variable: &mut crate::types::variable::ReflectInterfaceVariable,
        type_decorations: &Decorations,
        type_description: &crate::types::ReflectTypeDescription,
    ) -> Result<(), String> {
        if let Some(type_node_index) = self.find_node(type_description.id) {
            let type_node = &self.nodes[type_node_index];

            variable.members.reserve(type_description.members.len());
            for member_index in 0..type_node.member_count as usize {
                let member_decorations = &type_node.member_decorations[member_index];
                let member_type = &type_description.members[member_index];
                let mut member_variable = crate::types::ReflectInterfaceVariable::default();
                self.parse_interface_variable(
                    &module,
                    built_in,
                    &mut member_variable,
                    &member_decorations,
                    &member_type,
                )?;
                variable.members.push(member_variable);
            }

            if *type_description.op == spirv_headers::Op::TypeArray {
                variable.array = type_description.traits.array.clone();
            }

            if let Some(ref built_in) = type_decorations.built_in {
                variable.built_in = Some(crate::types::ReflectBuiltIn(*built_in));
            }

            variable.name = type_node.name.to_owned();
            variable.decoration_flags = Self::apply_decorations(&type_decorations)?;
            variable.numeric = type_description.traits.numeric.clone();
            //variable.format = Self::parse_format(&type_description)?;
            variable.type_description = type_description.to_owned();
        } else {
            return Err("Invalid SPIR-V ID reference".into());
        }

        Ok(())
    }

    fn parse_interface_variables(
        &self,
        _spv_words: &[u32],
        module: &mut super::ShaderModule,
        interface_vars: &[u32],
        entry_point: &mut crate::types::variable::ReflectEntryPoint,
    ) -> Result<(), String> {
        if interface_vars.len() > 0 {
            let mut input_count = 0;
            let mut output_count = 0;
            for var_id in interface_vars {
                if let Some(node_index) = self.find_node(*var_id) {
                    let node = &self.nodes[node_index];
                    match node.storage_class {
                        spirv_headers::StorageClass::Input => input_count += 1,
                        spirv_headers::StorageClass::Output => output_count += 1,
                        _ => {}
                    }
                } else {
                    return Err("Invalid SPIR-V ID reference".into());
                }
            }

            entry_point.input_variables.reserve(input_count);
            entry_point.output_variables.reserve(output_count);

            for var_id in interface_vars {
                if let Some(node_index) = self.find_node(*var_id) {
                    let node = &self.nodes[node_index];
                    if let Some(type_index) = module.internal.find_type(node.type_id) {
                        let mut type_description = &module.internal.type_descriptions[type_index];

                        // Resolve pointer types
                        if *type_description.op == spirv_headers::Op::TypePointer {
                            if let Some(type_node_index) = self.find_node(type_description.id) {
                                let type_node = &self.nodes[type_node_index];
                                if let Some(pointer_type_index) =
                                    module.internal.find_type(type_node.type_id)
                                {
                                    type_description =
                                        &module.internal.type_descriptions[pointer_type_index];
                                } else {
                                    return Err("Invalid SPIR-V ID reference".into());
                                }
                            } else {
                                return Err("Invalid SPIR-V ID reference".into());
                            }
                        }

                        if let Some(type_node_index) = self.find_node(type_description.id) {
                            let type_node = &self.nodes[type_node_index];
                            let type_decorations = &type_node.decorations;

                            let mut variable =
                                crate::types::variable::ReflectInterfaceVariable::default();
                            match node.storage_class {
                                spirv_headers::StorageClass::Input => {
                                    variable.storage_class =
                                        crate::types::ReflectStorageClass::Input
                                }
                                spirv_headers::StorageClass::Output => {
                                    variable.storage_class =
                                        crate::types::ReflectStorageClass::Output
                                }
                                spirv_headers::StorageClass::Uniform => {
                                    variable.storage_class =
                                        crate::types::ReflectStorageClass::Uniform
                                }
                                spirv_headers::StorageClass::UniformConstant => {
                                    variable.storage_class =
                                        crate::types::ReflectStorageClass::UniformConstant
                                }
                                spirv_headers::StorageClass::StorageBuffer => {
                                    variable.storage_class =
                                        crate::types::ReflectStorageClass::StorageBuffer
                                }
                                _ => {
                                    return Err(format!(
                                        "Invalid SPIR-V ID storage class {:?}",
                                        node.storage_class
                                    ))
                                }
                            }

                            let mut built_in = node.decorations.built_in.is_some();
                            self.parse_interface_variable(
                                &module,
                                &mut built_in,
                                &mut variable,
                                &type_decorations,
                                &type_description,
                            )?;

                            variable.spirv_id = node.result_id;
                            variable.name = node.name.to_owned();
                            variable.semantic = node.decorations.semantic.value.to_owned();
                            if built_in {
                                variable.decoration_flags |=
                                    crate::types::ReflectDecorationFlags::BUILT_IN;
                            }
                            variable.location = node.decorations.location.value;
                            variable.word_offset = node.decorations.location.word_offset;
                            if let Some(built_in) = node.decorations.built_in {
                                variable.built_in = Some(crate::types::ReflectBuiltIn(built_in));
                            }

                            match variable.storage_class {
                                crate::types::ReflectStorageClass::Input => {
                                    entry_point.input_variables.push(variable)
                                }
                                crate::types::ReflectStorageClass::Output => {
                                    entry_point.output_variables.push(variable)
                                }
                                _ => {}
                            }
                        } else {
                            return Err("Invalid SPIR-V ID reference".into());
                        }
                    } else {
                        return Err("Invalid SPIR-V ID reference".into());
                    }
                } else {
                    return Err("Invalid SPIR-V ID reference".into());
                }
            }
        }
        Ok(())
    }

    fn parse_static_resources(
        &self,
        _spv_words: &[u32],
        module: &mut super::ShaderModule,
        uniforms: &[u32],
        push_constants: &[u32],
        entry_point: &mut crate::types::variable::ReflectEntryPoint,
    ) -> Result<(), String> {
        for function_index in 0..self.functions.len() {
            if self.functions[function_index].id == entry_point.id {
                let mut called_functions = Vec::new();
                self.traverse_call_graph(function_index, &mut called_functions, 0)?;
                called_functions.sort();
                called_functions.dedup();

                let mut usage_count = 0;
                let mut check_index = 0;
                for called_index in 0..called_functions.len() {
                    while self.functions[check_index].id != called_functions[called_index] {
                        check_index += 1;
                    }

                    usage_count += self.functions[check_index].accessed.len();
                }

                // Used variables
                let mut usage: Vec<u32> = Vec::with_capacity(usage_count);

                check_index = 0;
                for called_index in 0..called_functions.len() {
                    while self.functions[check_index].id != called_functions[called_index] {
                        check_index += 1;
                    }

                    usage.extend(&self.functions[check_index].accessed);
                }

                usage.sort();
                usage.dedup();

                entry_point.used_uniforms =
                    uniforms.intersect(&usage).into_iter().map(|x| *x).collect();
                entry_point.used_push_constants = push_constants
                    .intersect(&usage)
                    .into_iter()
                    .map(|x| *x)
                    .collect();

                for binding_index in 0..module.internal.descriptor_bindings.len() {
                    let mut descriptor_binding =
                        &mut module.internal.descriptor_bindings[binding_index];
                    if usage
                        .iter()
                        .position(|x| x == &descriptor_binding.spirv_id)
                        .is_some()
                    {
                        descriptor_binding.accessed = true;
                    }
                }

                return Ok(());
            }
        }

        return Err("Invalid SPIR-V ID reference".into());
    }

    fn parse_entry_points(
        &mut self,
        spv_words: &[u32],
        module: &mut super::ShaderModule,
    ) -> Result<(), String> {
        module.internal.entry_points.reserve(self.entry_point_count);
        let uniforms = Self::enumerate_all_uniforms(module);
        let push_constants = Self::enumerate_all_push_constants(module);

        for node in &self.nodes {
            if node.op != spirv_headers::Op::EntryPoint {
                continue;
            }

            let word_offset = node.word_offset as usize;
            let word_count = node.word_count as usize;

            let spirv_execution_model =
                spirv_headers::ExecutionModel::from_u32(spv_words[word_offset + 1]);
            let shader_stage = match spirv_execution_model {
                Some(spirv_headers::ExecutionModel::Vertex) => {
                    crate::types::ReflectShaderStage::Vertex
                }
                Some(spirv_headers::ExecutionModel::TessellationControl) => {
                    crate::types::ReflectShaderStage::TessellationControl
                }
                Some(spirv_headers::ExecutionModel::TessellationEvaluation) => {
                    crate::types::ReflectShaderStage::TessellationEvaluation
                }
                Some(spirv_headers::ExecutionModel::Geometry) => {
                    crate::types::ReflectShaderStage::Geometry
                }
                Some(spirv_headers::ExecutionModel::Fragment) => {
                    crate::types::ReflectShaderStage::Fragment
                }
                Some(spirv_headers::ExecutionModel::GLCompute) => {
                    crate::types::ReflectShaderStage::Compute
                }
                Some(spirv_headers::ExecutionModel::Kernel) => {
                    crate::types::ReflectShaderStage::Kernel
                }
                _ => {
                    // TODO: Get NV support in spirv_headers. For now, parse it directly from raw
                    // https://www.khronos.org/registry/spir-v/specs/unified1/SPIRV.html#_a_id_execution_model_a_execution_model
                    match spv_words[word_offset + 1] {
                        5267 => crate::types::ReflectShaderStage::TaskNV,
                        5268 => crate::types::ReflectShaderStage::MeshNV,
                        5313 => crate::types::ReflectShaderStage::RayGenerationNV,
                        5314 => crate::types::ReflectShaderStage::IntersectionNV,
                        5315 => crate::types::ReflectShaderStage::AnyHitNV,
                        5316 => crate::types::ReflectShaderStage::ClosestHitNV,
                        5317 => crate::types::ReflectShaderStage::MissNV,
                        5318 => crate::types::ReflectShaderStage::CallableNV,
                        _ => crate::types::ReflectShaderStage::Undefined,
                    }
                }
            };

            // The name string length determines the next operand offset.
            let name_start_offset = 3;
            let name = unsafe {
                let name_offset = word_offset + name_start_offset;
                if name_offset + word_count >= spv_words.len() {
                    return Err("Count mismatch while parsing strings.".into());
                }

                // We want to take a byte slice of the valid name string range, since we can't assume
                // it is a valid null terminated string.
                let name_ptr = spv_words.as_ptr().offset(name_offset as isize) as *const _;
                let name_slice = std::slice::from_raw_parts(name_ptr, word_count * SPIRV_WORD_SIZE);
                let name_slice_end = name_slice.iter().position(|&b| b == 0).map_or(0, |i| i + 1);

                // Convert the slice to a string (if it's corectly null terminated).
                let name_str = CStr::from_bytes_with_nul(&name_slice[..name_slice_end]);
                if name_str.is_err() {
                    return Err("Entry point name is not a valid string.".into());
                }
                let name_str = name_str.unwrap();

                // Convert ffi to string
                name_str.to_str().unwrap().to_owned()
            };

            let name_length_with_null = name.len() + 1;
            let name_word_count = (name_length_with_null + SPIRV_WORD_SIZE - 1) / SPIRV_WORD_SIZE;

            let mut entry_point = crate::types::variable::ReflectEntryPoint {
                name,
                spirv_execution_model,
                id: spv_words[word_offset + 2],
                shader_stage,
                input_variables: Vec::new(),
                output_variables: Vec::new(),
                descriptor_sets: Vec::new(),
                used_uniforms: Vec::new(),
                used_push_constants: Vec::new(),
            };

            let interface_var_count = word_count - (name_start_offset + name_word_count);
            let interface_var_offset = name_start_offset + name_word_count;
            let mut interface_vars = Vec::with_capacity(interface_var_count);
            for var_index in 0..interface_var_count {
                let var_offset = interface_var_offset + var_index;
                interface_vars.push(spv_words[word_offset + var_offset]);
            }

            self.parse_interface_variables(spv_words, module, &interface_vars, &mut entry_point)?;
            self.parse_static_resources(
                spv_words,
                module,
                &uniforms,
                &push_constants,
                &mut entry_point,
            )?;

            module.internal.entry_points.push(entry_point);
        }

        Ok(())
    }

    fn traverse_call_graph(
        &self,
        function_index: usize,
        functions: &mut Vec<u32>,
        depth: usize,
    ) -> Result<(), String> {
        if depth > self.functions.len() {
            // Vulkan doesn't allow for recursion:
            // "Recursion: The static function-call graph for an entry point must not contain cycles."
            return Err("Entry point call graph must not contain cycles".into());
        }

        functions.push(self.functions[function_index].id);
        for callee in &self.functions[function_index].callees {
            self.traverse_call_graph(callee.function, functions, depth + 1)?;
        }
        Ok(())
    }

    fn find_node(&self, result_id: u32) -> Option<usize> {
        for node_index in 0..self.nodes.len() {
            let node = &self.nodes[node_index];
            if node.result_id == result_id {
                return Some(node_index);
            }
        }

        None
    }

    fn enumerate_all_uniforms(module: &super::ShaderModule) -> Vec<u32> {
        let mut uniforms: Vec<u32> = Vec::new();

        if module.internal.descriptor_bindings.len() > 0 {
            uniforms.reserve(module.internal.descriptor_bindings.len());
            for descriptor_binding in &module.internal.descriptor_bindings {
                uniforms.push(descriptor_binding.spirv_id);
            }

            uniforms.sort_by(|a, b| a.cmp(b));
        }

        uniforms
    }

    fn enumerate_all_push_constants(module: &super::ShaderModule) -> Vec<u32> {
        let mut push_constants: Vec<u32> = Vec::new();

        if module.internal.push_constant_blocks.len() > 0 {
            push_constants.reserve(module.internal.push_constant_blocks.len());
            for push_constant_block in &module.internal.push_constant_blocks {
                push_constants.push(push_constant_block.spirv_id);
            }

            push_constants.sort_by(|a, b| a.cmp(b));
        }

        push_constants
    }
}

pub trait IterOps<T, I>: IntoIterator<Item = T>
where
    I: IntoIterator<Item = T>,
    T: PartialEq,
{
    fn intersect(self, other: I) -> Vec<T>;
    fn difference(self, other: I) -> Vec<T>;
}

impl<T, I> IterOps<T, I> for I
where
    I: IntoIterator<Item = T>,
    T: PartialEq,
{
    fn intersect(self, other: I) -> Vec<T> {
        let mut common = Vec::new();
        let mut v_other: Vec<_> = other.into_iter().collect();

        for e1 in self.into_iter() {
            if let Some(pos) = v_other.iter().position(|e2| e1 == *e2) {
                common.push(e1);
                v_other.remove(pos);
            }
        }

        common
    }

    fn difference(self, other: I) -> Vec<T> {
        let mut diff = Vec::new();
        let mut v_other: Vec<_> = other.into_iter().collect();

        for e1 in self.into_iter() {
            if let Some(pos) = v_other.iter().position(|e2| e1 == *e2) {
                v_other.remove(pos);
            } else {
                diff.push(e1);
            }
        }

        diff.append(&mut v_other);
        diff
    }
}
