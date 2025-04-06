use wesl_docs::*;

pub fn post_process(docs: &mut WeslDocs) {
    // Post process modules
    post_process_module(&mut docs.root);
}

fn post_process_module(module: &mut Module) -> IndexSet<String> {
    // Inner modules
    for inner in &mut module.modules {
        let res_inner = post_process_module(inner);
        module.translate_time_features.extend(res_inner);
    }

    // Sort modules
    module.modules.sort_by(|a, b| a.name.cmp(&b.name));

    // Post process items
    post_process_items(&mut module.constants);
    post_process_items(&mut module.global_variables);
    post_process_items(&mut module.structs);
    post_process_items(&mut module.functions);
    post_process_items(&mut module.type_aliases);

    // Sort shader defs
    module.translate_time_features.sort();

    module.translate_time_features.clone()
}

fn post_process_items<T: ItemInstance>(items: &mut IndexMap<Ident, Item<T>>) {
    // Sort
    items.sort_keys();

    // Post process each item
    for item in items.values_mut() {
        item.conditional = item_conditional(item);
    }
}

fn item_conditional<T: ItemInstance>(item: &Item<T>) -> Option<Conditional> {
    let mut acc = item.instances[0].conditional()?.clone();
    for instance in &item.instances[1..] {
        let cond = instance.conditional()?.clone();
        acc = Conditional::Or(Box::new(acc), Box::new(cond));
    }

    if is_tautology(&acc) {
        return None;
    }

    Some(acc)
}

fn is_tautology(cond: &Conditional) -> bool {
    // Collect all features from the conditional
    let mut features = IndexMap::new();
    collect_features(cond, &mut features);

    // Do not attempt to evaluate conditionals with more than 8 features
    if features.len() > 8 {
        return false;
    }

    // Solve all possible combinations of features
    for mask in 0..(1u32 << features.len()) {
        // Set the features from the mask
        for (i, value) in features.values_mut().enumerate() {
            *value = (mask >> i) & 1 == 1;
        }

        // Evaluate the conditional. If it is false, we can skip the rest.
        if !evaluate_conditional(cond, &features) {
            return false;
        }
    }

    // All evaluations returned true, so it is a tautology
    true
}

fn collect_features(cond: &Conditional, features: &mut IndexMap<Ident, bool>) {
    match cond {
        Conditional::False => (),
        Conditional::True => (),
        Conditional::Feature(ident) => {
            features.insert(ident.clone(), false);
        }
        Conditional::Not(operand) => collect_features(operand, features),
        Conditional::And(left, right) | Conditional::Or(left, right) => {
            collect_features(left, features);
            collect_features(right, features);
        }
    }
}

fn evaluate_conditional(cond: &Conditional, features: &IndexMap<Ident, bool>) -> bool {
    match cond {
        Conditional::False => false,
        Conditional::True => true,
        Conditional::Feature(ident) => features.get(ident).is_some_and(|v| *v),
        Conditional::Not(operand) => !evaluate_conditional(operand, features),
        Conditional::And(left, right) => {
            evaluate_conditional(left, features) && evaluate_conditional(right, features)
        }
        Conditional::Or(left, right) => {
            evaluate_conditional(left, features) || evaluate_conditional(right, features)
        }
    }
}
