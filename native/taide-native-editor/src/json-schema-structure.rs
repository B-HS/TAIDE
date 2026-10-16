use std::collections::BTreeMap;

use crate::document::EditorError;
use crate::json_parser::{Kind, Value as AstValue};
use crate::json_schema_evaluation::{
    Collection, Contains, Draft, Evaluator, Frame, Merge, Operation, Property,
};
use crate::json_schema_string::compile_extended;
use crate::json_value::Value;

impl Evaluator<'_> {
    fn numeric_problem(
        &mut self,
        frame: &mut Frame,
        prefix: &str,
        value: f64,
        suffix: &str,
        custom: bool,
    ) -> Result<(), EditorError> {
        let message = if custom {
            self.custom(frame.schema)?
        } else {
            None
        };
        let message = if let Some(message) = message {
            message
        } else {
            self.text(
                prefix,
                &crate::json_symbols::number_string(value)
                    .encode_utf16()
                    .collect::<Vec<_>>(),
                suffix,
            )?
        };
        self.problem(frame, frame.node, message, false)
    }

    fn array(&mut self, frame: &mut Frame) -> Result<(), EditorError> {
        let items = &self.document.nodes[frame.node].children;
        if items.len() > self.remaining {
            return Err(EditorError::Capacity);
        }
        let item_schema = self.field(frame.schema, "items")?;
        let is_tuple = item_schema
            .is_some_and(|index| matches!(self.schema.get(index), Some(Value::Array(_))));
        let prefix = if self.draft >= Draft::Draft2020 {
            self.field(frame.schema, "prefixItems")?
        } else if is_tuple {
            item_schema
        } else {
            None
        };
        let additional = if self.draft >= Draft::Draft2020 || !is_tuple {
            item_schema.filter(|_| !is_tuple)
        } else {
            self.field(frame.schema, "additionalItems")?
        };
        let mut index = 0;
        let mut operations = Vec::new();
        if let Some(prefix) = prefix {
            let Some(Value::Array(schemas)) = self.schema.get(prefix) else {
                return Err(EditorError::InvalidBoundary);
            };
            for (&node, &schema) in items.iter().zip(schemas) {
                operations.push(Operation::Child {
                    node,
                    schema,
                    collect: frame.collect,
                    merge: Merge::Property(Some(index.to_string().encode_utf16().collect())),
                });
                index += 1;
            }
        }
        if let Some(schema) = additional.filter(|_| index < items.len()) {
            match self.schema.get(schema) {
                Some(Value::Boolean(value)) => {
                    if !value {
                        let message = self.text(
                            "Array has too many items according to schema. Expected ",
                            &index.to_string().encode_utf16().collect::<Vec<_>>(),
                            " or fewer.",
                        )?;
                        operations.push(Operation::Problem(
                            self.diagnostic(frame.node, message, false)?,
                        ));
                    }
                    for index in index..items.len() {
                        operations.push(Operation::Mark {
                            name: index.to_string().encode_utf16().collect(),
                            value_match: true,
                        });
                    }
                }
                Some(_) => {
                    for (index, &node) in items.iter().enumerate().skip(index) {
                        operations.push(Operation::Child {
                            node,
                            schema,
                            collect: frame.collect,
                            merge: Merge::Property(Some(
                                index.to_string().encode_utf16().collect(),
                            )),
                        });
                    }
                }
                None => return Err(EditorError::InvalidIdentity),
            }
        }
        let contains = self.field(frame.schema, "contains")?;
        if let Some(schema) = self.schema_ref(contains)? {
            operations.push(Operation::Contains(Contains {
                schema,
                index: 0,
                matches: 0,
            }));
        }
        operations.push(Operation::UnevaluatedArray);
        operations.push(Operation::ArrayBounds);
        self.queue(frame, operations)
    }

    fn contains(&mut self, frame: &mut Frame, mut contains: Contains) -> Result<(), EditorError> {
        let items = &self.document.nodes[frame.node].children;
        if let Some(&node) = items.get(contains.index) {
            contains.index += 1;
            return self.queue(
                frame,
                vec![Operation::Child {
                    node,
                    schema: contains.schema,
                    collect: Collection::disabled(),
                    merge: Merge::Contains(contains),
                }],
            );
        }
        let min = self.number(frame.schema, "minContains")?;
        let max = self.number(frame.schema, "maxContains")?;
        if contains.matches == 0 && min.is_none() {
            let message = if let Some(message) = self.custom(frame.schema)? {
                message
            } else {
                self.text("Array does not contain required item.", &[], "")?
            };
            self.problem(frame, frame.node, message, false)?;
        }
        if let Some(min) = min.filter(|min| (contains.matches as f64) < *min) {
            self.numeric_problem(
                frame,
                "Array has too few items that match the contains contraint. Expected ",
                min,
                " or more.",
                true,
            )?;
        }
        if let Some(max) = max.filter(|max| (contains.matches as f64) > *max) {
            self.numeric_problem(
                frame,
                "Array has too many items that match the contains contraint. Expected ",
                max,
                " or less.",
                true,
            )?;
        }
        Ok(())
    }

    fn unevaluated_array(&mut self, frame: &mut Frame) -> Result<(), EditorError> {
        let Some(schema) = self.field(frame.schema, "unevaluatedItems")? else {
            return Ok(());
        };
        let items = &self.document.nodes[frame.node].children;
        if items.len() > self.remaining {
            return Err(EditorError::Capacity);
        }
        let mut operations = Vec::new();
        for (index, &node) in items.iter().enumerate() {
            let name = index.to_string().encode_utf16().collect::<Vec<_>>();
            if !frame.result.processed_properties.contains(&name) {
                if matches!(self.schema.get(schema), Some(Value::Boolean(false))) {
                    let message = self.text(
                        "Item does not match any validation rule from the array.",
                        &[],
                        "",
                    )?;
                    self.problem(frame, frame.node, message, false)?;
                } else {
                    operations.push(Operation::Child {
                        node,
                        schema,
                        collect: frame.collect,
                        merge: Merge::RawProperty,
                    });
                }
            }
            operations.push(Operation::Mark {
                name,
                value_match: true,
            });
        }
        self.queue(frame, operations)
    }

    fn array_bounds(&mut self, frame: &mut Frame) -> Result<(), EditorError> {
        let items = &self.document.nodes[frame.node].children;
        if let Some(min) = self
            .number(frame.schema, "minItems")?
            .filter(|min| (items.len() as f64) < *min)
        {
            self.numeric_problem(
                frame,
                "Array has too few items. Expected ",
                min,
                " or more.",
                false,
            )?;
        }
        if let Some(max) = self
            .number(frame.schema, "maxItems")?
            .filter(|max| (items.len() as f64) > *max)
        {
            self.numeric_problem(
                frame,
                "Array has too many items. Expected ",
                max,
                " or fewer.",
                false,
            )?;
        }
        let unique = self
            .field(frame.schema, "uniqueItems")?
            .is_some_and(|index| matches!(self.schema.get(index), Some(Value::Boolean(true))));
        if !unique {
            return Ok(());
        }
        for (index, &one) in items.iter().enumerate() {
            for &two in &items[index + 1..] {
                let one = self.mapped[one].ok_or(EditorError::InvalidIdentity)?;
                let two = self.mapped[two].ok_or(EditorError::InvalidIdentity)?;
                if self
                    .values
                    .equals_budgeted(one, self.values, two, &mut self.remaining)?
                {
                    let message = self.text("Array has duplicate items.", &[], "")?;
                    return self.problem(frame, frame.node, message, false);
                }
            }
        }
        Ok(())
    }

    fn object(&mut self, frame: &mut Frame) -> Result<(), EditorError> {
        let properties = &self.document.nodes[frame.node].children;
        if properties.len() > self.remaining {
            return Err(EditorError::Capacity);
        }
        let mut positions = BTreeMap::new();
        for &property in properties {
            self.spend(1)?;
            let property = &self.document.nodes[property];
            let key_node = *property
                .children
                .first()
                .ok_or(EditorError::InvalidBoundary)?;
            let AstValue::String(name) = &self.document.nodes[key_node].value else {
                return Err(EditorError::InvalidBoundary);
            };
            let value = property.children.get(1).copied();
            self.spend(name.len() + 1)?;
            if let Some(&index) = positions.get(name.as_slice()) {
                let property: &mut Property = &mut frame.properties[index];
                property.value = value;
                property.key_node = key_node;
            } else {
                positions.insert(name.as_slice(), frame.properties.len());
                frame.properties.push(Property {
                    name: name.clone(),
                    value,
                    key_node,
                    processed: false,
                });
            }
        }
        if let Some(required) = self.field(frame.schema, "required")? {
            if let Some(Value::Array(required)) = self.schema.get(required) {
                for &name in required {
                    let Some(Value::String(name)) = self.schema.get(name) else {
                        return Err(EditorError::InvalidBoundary);
                    };
                    self.spend(name.len() + frame.properties.len() + 1)?;
                    if !frame
                        .properties
                        .iter()
                        .any(|property| property.name == *name && property.value.is_some())
                    {
                        let parent_key = self.document.nodes[frame.node]
                            .parent
                            .and_then(|parent| self.document.nodes.get(parent))
                            .filter(|parent| parent.kind == Kind::Property)
                            .and_then(|parent| parent.children.first())
                            .copied();
                        let message = self.text("Missing property \"", name, "\".")?;
                        self.problem(
                            frame,
                            parent_key.unwrap_or(frame.node),
                            message,
                            parent_key.is_none(),
                        )?;
                    }
                }
            }
        }
        let mut operations = Vec::new();
        if let Some(properties) = self.field(frame.schema, "properties")? {
            if let Some(Value::Object(properties)) = self.schema.get(properties) {
                if properties.len() > self.remaining {
                    return Err(EditorError::Capacity);
                }
                for property in properties {
                    self.process_property(
                        frame,
                        &property.key,
                        property.value,
                        &mut operations,
                        true,
                    )?;
                }
            }
        }
        operations.push(Operation::PatternProperties(0));
        operations.push(Operation::AdditionalProperties(false));
        operations.push(Operation::AdditionalProperties(true));
        operations.push(Operation::ObjectBounds);
        operations.push(Operation::Dependencies("dependentRequired"));
        operations.push(Operation::Dependencies("dependentSchemas"));
        operations.push(Operation::Dependencies("dependencies"));
        operations.push(Operation::PropertyNames);
        self.queue(frame, operations)
    }

    fn process_property(
        &mut self,
        frame: &mut Frame,
        name: &[u16],
        schema: usize,
        operations: &mut Vec<Operation>,
        count_true: bool,
    ) -> Result<(), EditorError> {
        self.spend(name.len() + frame.properties.len() + 1)?;
        self.mark(&mut frame.result, name.to_vec())?;
        let Some(index) = frame
            .properties
            .iter()
            .position(|property| property.name == name)
        else {
            return Ok(());
        };
        frame.properties[index].processed = true;
        let Some(node) = frame.properties[index].value else {
            return Ok(());
        };
        match self
            .schema
            .get(schema)
            .ok_or(EditorError::InvalidIdentity)?
        {
            Value::Boolean(false) => {
                let message = if let Some(message) = self.custom(frame.schema)? {
                    message
                } else {
                    self.text("Property ", name, " is not allowed.")?
                };
                operations.push(Operation::Problem(self.diagnostic(
                    frame.properties[index].key_node,
                    message,
                    false,
                )?));
            }
            Value::Boolean(true) => {
                if count_true {
                    frame.result.properties_matches += 1;
                    frame.result.properties_value_matches += 1;
                }
            }
            _ => operations.push(Operation::Child {
                node,
                schema,
                collect: frame.collect,
                merge: Merge::Property(None),
            }),
        }
        Ok(())
    }

    fn pattern_properties(&mut self, frame: &mut Frame, index: usize) -> Result<(), EditorError> {
        let Some(patterns) = self.field(frame.schema, "patternProperties")? else {
            return Ok(());
        };
        let Some(Value::Object(patterns)) = self.schema.get(patterns) else {
            return Ok(());
        };
        let Some(pattern) = patterns.get(index) else {
            return Ok(());
        };
        let mut operations = Vec::new();
        if let Some(regex) = compile_extended(self.compiler, &pattern.key, &mut self.remaining)? {
            for index in 0..frame.properties.len() {
                if frame.properties[index].processed {
                    continue;
                }
                self.spend(frame.properties[index].name.len() + 1)?;
                if regex.is_match(&frame.properties[index].name) {
                    let name = frame.properties[index].name.clone();
                    self.process_property(frame, &name, pattern.value, &mut operations, true)?;
                }
            }
        }
        operations.push(Operation::PatternProperties(index + 1));
        self.queue(frame, operations)
    }

    fn additional_properties(
        &mut self,
        frame: &mut Frame,
        unevaluated: bool,
    ) -> Result<(), EditorError> {
        let Some(schema) = self.field(
            frame.schema,
            if unevaluated {
                "unevaluatedProperties"
            } else {
                "additionalProperties"
            },
        )?
        else {
            return Ok(());
        };
        let mut operations = Vec::new();
        for index in 0..frame.properties.len() {
            self.spend(1)?;
            let property = &frame.properties[index];
            if property.processed
                || unevaluated && frame.result.processed_properties.contains(&property.name)
            {
                continue;
            }
            let name = property.name.clone();
            self.process_property(frame, &name, schema, &mut operations, false)?;
        }
        self.queue(frame, operations)
    }

    fn object_bounds(&mut self, frame: &mut Frame) -> Result<(), EditorError> {
        let count = self.document.nodes[frame.node].children.len();
        if let Some(max) = self
            .number(frame.schema, "maxProperties")?
            .filter(|max| (count as f64) > *max)
        {
            self.numeric_problem(
                frame,
                "Object has more properties than limit of ",
                max,
                ".",
                false,
            )?;
        }
        if let Some(min) = self
            .number(frame.schema, "minProperties")?
            .filter(|min| (count as f64) < *min)
        {
            self.numeric_problem(
                frame,
                "Object has fewer properties than the required number of ",
                min,
                "",
                false,
            )?;
        }
        Ok(())
    }

    fn dependencies(&mut self, frame: &mut Frame, keyword: &str) -> Result<(), EditorError> {
        let Some(index) = self.field(frame.schema, keyword)? else {
            return Ok(());
        };
        let Some(Value::Object(dependencies)) = self.schema.get(index) else {
            return Ok(());
        };
        if dependencies.len() > self.remaining {
            return Err(EditorError::Capacity);
        }
        let mut operations = Vec::new();
        for dependency in dependencies {
            self.spend(frame.properties.len() + dependency.key.len() + 1)?;
            if !frame
                .properties
                .iter()
                .any(|property| property.name == dependency.key && property.value.is_some())
            {
                continue;
            }
            let value = self
                .schema
                .get(dependency.value)
                .ok_or(EditorError::InvalidIdentity)?;
            if keyword == "dependentRequired" && !matches!(value, Value::Array(_))
                || keyword == "dependentSchemas" && !matches!(value, Value::Object(_))
            {
                continue;
            }
            if let Value::Array(required) = value {
                for &name in required {
                    let Some(Value::String(name)) = self.schema.get(name) else {
                        return Err(EditorError::InvalidBoundary);
                    };
                    self.spend(frame.properties.len() + name.len() + 1)?;
                    if frame
                        .properties
                        .iter()
                        .any(|property| property.name == *name && property.value.is_some())
                    {
                        frame.result.properties_value_matches += 1;
                    } else {
                        let mut message = self.text(
                            "Object is missing property ",
                            name,
                            " required by property ",
                        )?;
                        let tail = self.text("", &dependency.key, ".")?;
                        message.extend(tail);
                        operations.push(Operation::Problem(
                            self.diagnostic(frame.node, message, false)?,
                        ));
                    }
                }
            } else if let Some(schema) = self.schema_ref(Some(dependency.value))? {
                operations.push(Operation::Child {
                    node: frame.node,
                    schema,
                    collect: frame.collect,
                    merge: Merge::Property(None),
                });
            }
        }
        self.queue(frame, operations)
    }

    fn property_names(&mut self, frame: &mut Frame) -> Result<(), EditorError> {
        let property_schema = self.field(frame.schema, "propertyNames")?;
        let Some(schema) = self.schema_ref(property_schema)? else {
            return Ok(());
        };
        let properties = &self.document.nodes[frame.node].children;
        if properties.len() > self.remaining {
            return Err(EditorError::Capacity);
        }
        let mut operations = Vec::new();
        for &property in properties {
            let node = *self.document.nodes[property]
                .children
                .first()
                .ok_or(EditorError::InvalidBoundary)?;
            operations.push(Operation::Child {
                node,
                schema,
                collect: Collection::disabled(),
                merge: Merge::Direct,
            });
        }
        self.queue(frame, operations)
    }

    pub(crate) fn structural_operation(
        &mut self,
        frame: &mut Frame,
        operation: Operation,
    ) -> Result<(), EditorError> {
        match operation {
            Operation::Structure => match self.document.nodes[frame.node].kind {
                Kind::Array => self.array(frame)?,
                Kind::Object => self.object(frame)?,
                _ => {}
            },
            Operation::Contains(contains) => self.contains(frame, contains)?,
            Operation::UnevaluatedArray => self.unevaluated_array(frame)?,
            Operation::ArrayBounds => self.array_bounds(frame)?,
            Operation::Mark { name, value_match } => {
                self.mark(&mut frame.result, name)?;
                if value_match {
                    frame.result.properties_value_matches += 1;
                }
            }
            Operation::PatternProperties(index) => self.pattern_properties(frame, index)?,
            Operation::AdditionalProperties(unevaluated) => {
                self.additional_properties(frame, unevaluated)?
            }
            Operation::ObjectBounds => self.object_bounds(frame)?,
            Operation::Dependencies(keyword) => self.dependencies(frame, keyword)?,
            Operation::PropertyNames => self.property_names(frame)?,
            _ => return Err(EditorError::InvalidBoundary),
        }
        Ok(())
    }
}
