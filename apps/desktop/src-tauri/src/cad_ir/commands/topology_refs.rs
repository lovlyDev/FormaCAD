use super::super::*;

pub(super) fn apply(document: &mut Document, command: &Command) -> Result<()> {
    let Command::SetTopologyReference {
        feature_id,
        reference,
    } = command
    else {
        return Err(ValidationError::new(ErrorCode::UnsupportedField, None));
    };
    let feature = document
        .features
        .iter_mut()
        .find(|feature| feature.id == *feature_id)
        .ok_or_else(|| ValidationError::new(ErrorCode::MissingTarget, Some(feature_id.clone())))?;
    let Operation::FilletReferencedEdge {
        reference: current, ..
    } = &mut feature.operation
    else {
        return Err(ValidationError::new(
            ErrorCode::UnsupportedField,
            Some(feature_id.clone()),
        ));
    };
    *current = reference.clone();
    Ok(())
}
