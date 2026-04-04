use crate::diagnostics::source_diagnostic_for_span;
use crate::diagnostics::source_excerpt_for_span;
use ocelot_ast::effect_index::EffectIndex;
use ocelot_ast::function_index::FunctionIndex;
use ocelot_base::compilation_context::CompilationContext;
use ocelot_base::result::OcelotResult;
use ocelot_base::shared_string::SharedString;
use ocelot_base::span::Span;
use ocelot_semantic::function_definition::FunctionDefinition;
use ocelot_semantic::function_kind::FunctionKind;
use ocelot_semantic::program_index::ProgramIndex;
use ocelot_semantic::resolved_function::ResolvedFunction;
use std::collections::HashMap;

pub(crate) fn propagate_function_effects(
    compilation_context: &mut CompilationContext,
    program_index: &ProgramIndex,
    resolved_functions: &mut [ResolvedFunction],
) -> OcelotResult<()> {
    let resolved_function_map = resolved_functions
        .iter()
        .enumerate()
        .map(|(index, function)| (function.function_index, index))
        .collect::<HashMap<_, _>>();

    let mut changed = true;
    while changed {
        changed = false;
        let inferred_effects_by_function = resolved_functions
            .iter()
            .map(|function| (function.function_index, function.inferred_effects.clone()))
            .collect::<HashMap<_, _>>();

        for function in resolved_functions.iter_mut() {
            let current_inferred_effects = function.inferred_effects.clone();
            let mut next_effects = function.direct_effects.clone();

            for called_function_index in function.called_functions.keys() {
                if let Some(inferred_effects) =
                    inferred_effects_by_function.get(called_function_index)
                {
                    next_effects.extend(inferred_effects.iter().copied());
                } else {
                    let called_function =
                        program_index.function_definition(*called_function_index)?;
                    next_effects.extend(called_function.inferred_effects.iter().copied());
                }
            }

            if next_effects != current_inferred_effects {
                function.inferred_effects = next_effects;
                changed = true;
            }
        }
    }

    for function in resolved_functions.iter() {
        let function_definition = program_index.function_definition(function.function_index)?;

        for forbidden_effect in &function_definition.cannot_effects {
            if !function.inferred_effects.contains(forbidden_effect) {
                continue;
            }

            let effect_name = program_index
                .effect_definition(*forbidden_effect)?
                .name
                .clone();
            let Some((span, annotation)) = violation_source(
                program_index,
                function_definition,
                function,
                resolved_functions,
                &resolved_function_map,
                *forbidden_effect,
            )?
            else {
                continue;
            };

            let FunctionKind::UserDefined { source_file, .. } = &function_definition.kind else {
                continue;
            };

            let mut diagnostic = source_diagnostic_for_span(
                source_file,
                format!(
                    "effect error: function `{}` cannot perform effect `{}`",
                    function_definition.name, effect_name
                ),
                span,
                annotation,
            );

            if let Some(cannot_clause_span) = function_definition.cannot_clause_span.clone() {
                diagnostic = diagnostic.with_excerpt(source_excerpt_for_span(
                    source_file,
                    cannot_clause_span,
                    "forbidden here",
                ));
            }

            compilation_context.add_diagnostic(diagnostic);
        }
    }

    Ok(())
}

fn violation_source(
    program_index: &ProgramIndex,
    function_definition: &FunctionDefinition,
    resolved_function: &ResolvedFunction,
    resolved_functions: &[ResolvedFunction],
    resolved_function_map: &HashMap<FunctionIndex, usize>,
    effect_index: EffectIndex,
) -> OcelotResult<Option<(Span, SharedString)>> {
    if function_definition.can_effects.contains(&effect_index)
        && let Some(span) = function_definition.can_clause_span.clone()
    {
        return Ok(Some((span, "effect declared here".into())));
    }

    for (called_function_index, span) in &resolved_function.called_functions {
        let called_has_effect =
            if let Some(resolved_index) = resolved_function_map.get(called_function_index) {
                resolved_functions[*resolved_index]
                    .inferred_effects
                    .contains(&effect_index)
            } else {
                program_index
                    .function_definition(*called_function_index)?
                    .inferred_effects
                    .contains(&effect_index)
            };
        if called_has_effect {
            return Ok(Some((
                span.clone(),
                format!(
                    "this has a `{}` effect",
                    effect_label(program_index, effect_index)?
                )
                .into(),
            )));
        }
    }

    if let Some(span) = resolved_function.direct_effect_sources.get(&effect_index) {
        return Ok(Some((
            span.clone(),
            format!(
                "this has a `{}` effect",
                effect_label(program_index, effect_index)?
            )
            .into(),
        )));
    }

    if let Some(span) = function_definition.cannot_clause_span.clone() {
        return Ok(Some((span, "forbidden effect".into())));
    }

    Ok(None)
}

fn effect_label(
    program_index: &ProgramIndex,
    effect_index: EffectIndex,
) -> OcelotResult<SharedString> {
    Ok(program_index.effect_definition(effect_index)?.name.clone())
}
