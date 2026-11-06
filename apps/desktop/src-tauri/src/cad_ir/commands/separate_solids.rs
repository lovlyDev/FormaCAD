use super::super::*;

pub(super) fn apply(document: &mut Document, command: &Command) -> Result<()> {
    let Command::SeparateSolids {
        body_id,
        new_bodies,
    } = command
    else {
        return Err(ValidationError::new(ErrorCode::UnsupportedField, None));
    };
    let invalid = |code| ValidationError::new(code, Some(body_id.clone()));
    if !(1..=127).contains(&new_bodies.len()) {
        return Err(invalid(ErrorCode::InvalidValue));
    }
    let prefix = &new_bodies[0].id;
    if prefix.len() > 64 || !super::valid_id(prefix) {
        return Err(invalid(ErrorCode::InvalidId));
    }
    let body = document
        .bodies
        .iter_mut()
        .find(|body| body.id == *body_id)
        .ok_or_else(|| invalid(ErrorCode::MissingTarget))?;
    let source = body.source_feature_id.clone();
    let count = (new_bodies.len() + 1) as u8;
    let feature = |index, name| Feature {
        id: format!("{prefix}_part{index}"),
        name,
        operation: Operation::ExtractSolid {
            body_feature_id: source.clone(),
            index,
            expected_count: count,
        },
        suppressed: false,
    };
    let first = feature(0, body.name.clone());
    body.source_feature_id = first.id.clone();
    document.features.push(first);
    for (index, new_body) in new_bodies.iter().enumerate() {
        let part = feature((index + 1) as u8, new_body.name.clone());
        document.bodies.push(Body {
            id: new_body.id.clone(),
            name: new_body.name.clone(),
            source_feature_id: part.id.clone(),
        });
        document.features.push(part);
    }
    Ok(())
}
