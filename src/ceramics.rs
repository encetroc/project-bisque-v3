//! Data model and validated recipe catalog for ceramic objects.

use crate::{inventory::CeramicObjectId, planet::ResourceType};

/// The three ceramic forms supported by the prototype.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CeramicForm {
    Cup,
    Bowl,
    Vase,
}

impl CeramicForm {
    pub const ALL: [Self; 3] = [Self::Cup, Self::Bowl, Self::Vase];
}

/// Clay varieties associated with the planet's resource nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClayMaterial {
    Common,
    Red,
    Pale,
}

impl ClayMaterial {
    pub const ALL: [Self; 3] = [Self::Common, Self::Red, Self::Pale];

    pub const fn resource(self) -> ResourceType {
        match self {
            Self::Common => ResourceType::CommonClay,
            Self::Red => ResourceType::RedClay,
            Self::Pale => ResourceType::PaleClay,
        }
    }
}

/// Glaze applied to a ceramic before it is fired.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Glaze {
    None,
    Blue,
    Green,
    White,
}

impl Glaze {
    pub const ALL: [Self; 4] = [Self::None, Self::Blue, Self::Green, Self::White];
}

/// Processing stage of a unique ceramic object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProcessingState {
    Greenware,
    Dry,
    Fired,
}

impl ProcessingState {
    const fn next(self) -> Option<Self> {
        match self {
            Self::Greenware => Some(Self::Dry),
            Self::Dry => Some(Self::Fired),
            Self::Fired => None,
        }
    }
}

/// A rejected processing transition, preserving both the requested and current stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidStageTransition {
    pub from: ProcessingState,
    pub to: ProcessingState,
}

/// The recipe-defined properties of a ceramic, before assigning a unique identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CeramicItemTemplate {
    pub form: CeramicForm,
    pub clay: ClayMaterial,
    pub glaze: Glaze,
    pub state: ProcessingState,
}

impl CeramicItemTemplate {
    pub const fn instantiate(self, id: CeramicObjectId) -> CeramicItem {
        CeramicItem { id, template: self }
    }
}

/// One uniquely identified ceramic, retaining all of its defining properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CeramicItem {
    pub id: CeramicObjectId,
    pub template: CeramicItemTemplate,
}

impl CeramicItem {
    pub const fn form(self) -> CeramicForm {
        self.template.form
    }

    pub const fn clay(self) -> ClayMaterial {
        self.template.clay
    }

    pub const fn glaze(self) -> Glaze {
        self.template.glaze
    }

    pub const fn state(self) -> ProcessingState {
        self.template.state
    }

    /// Advance exactly one processing stage; skipping, repeating, and reversing fail.
    pub fn transition_to(&mut self, to: ProcessingState) -> Result<(), InvalidStageTransition> {
        if self.template.state.next() == Some(to) {
            self.template.state = to;
            Ok(())
        } else {
            Err(InvalidStageTransition {
                from: self.template.state,
                to,
            })
        }
    }
}

/// One stackable resource consumed by a recipe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecipeInput {
    pub resource: ResourceType,
    pub quantity: u32,
}

/// The complete data-driven recipe for one form/material/glaze combination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CeramicRecipe {
    pub input: RecipeInput,
    pub output: CeramicItemTemplate,
}

const RECIPE_COUNT: usize = CeramicForm::ALL.len() * ClayMaterial::ALL.len() * Glaze::ALL.len();

const fn build_recipe_catalog() -> [CeramicRecipe; RECIPE_COUNT] {
    let mut recipes = [CeramicRecipe {
        input: RecipeInput {
            resource: ResourceType::CommonClay,
            quantity: 3,
        },
        output: CeramicItemTemplate {
            form: CeramicForm::Cup,
            clay: ClayMaterial::Common,
            glaze: Glaze::None,
            state: ProcessingState::Greenware,
        },
    }; RECIPE_COUNT];
    let mut index = 0;
    let mut form_index = 0;
    while form_index < CeramicForm::ALL.len() {
        let mut clay_index = 0;
        while clay_index < ClayMaterial::ALL.len() {
            let mut glaze_index = 0;
            while glaze_index < Glaze::ALL.len() {
                let clay = ClayMaterial::ALL[clay_index];
                recipes[index] = CeramicRecipe {
                    input: RecipeInput {
                        resource: clay.resource(),
                        quantity: 3,
                    },
                    output: CeramicItemTemplate {
                        form: CeramicForm::ALL[form_index],
                        clay,
                        glaze: Glaze::ALL[glaze_index],
                        state: ProcessingState::Greenware,
                    },
                };
                index += 1;
                glaze_index += 1;
            }
            clay_index += 1;
        }
        form_index += 1;
    }
    recipes
}

/// Complete recipe catalog: every form, clay, and glaze combination.
pub const CERAMIC_RECIPES: [CeramicRecipe; RECIPE_COUNT] = build_recipe_catalog();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogValidationError {
    MissingOrDuplicateCombination,
    InvalidInputQuantity,
    InvalidOutputStage,
    InputOutputClayMismatch,
}

/// Find the recipe whose output has the requested ceramic properties.
pub fn recipe_for(
    form: CeramicForm,
    clay: ClayMaterial,
    glaze: Glaze,
) -> Option<&'static CeramicRecipe> {
    CERAMIC_RECIPES.iter().find(|recipe| {
        recipe.output.form == form && recipe.output.clay == clay && recipe.output.glaze == glaze
    })
}

/// Check catalog cardinality, combination uniqueness, and input/output consistency.
pub fn validate_catalog() -> Result<(), CatalogValidationError> {
    let mut seen = [[[false; Glaze::ALL.len()]; ClayMaterial::ALL.len()]; CeramicForm::ALL.len()];
    for recipe in &CERAMIC_RECIPES {
        if recipe.input.quantity == 0 {
            return Err(CatalogValidationError::InvalidInputQuantity);
        }
        if recipe.output.state != ProcessingState::Greenware {
            return Err(CatalogValidationError::InvalidOutputStage);
        }
        if recipe.input.resource != recipe.output.clay.resource() {
            return Err(CatalogValidationError::InputOutputClayMismatch);
        }
        let form_index = CeramicForm::ALL
            .iter()
            .position(|form| *form == recipe.output.form)
            .ok_or(CatalogValidationError::MissingOrDuplicateCombination)?;
        let clay_index = ClayMaterial::ALL
            .iter()
            .position(|clay| *clay == recipe.output.clay)
            .ok_or(CatalogValidationError::MissingOrDuplicateCombination)?;
        let glaze_index = Glaze::ALL
            .iter()
            .position(|glaze| *glaze == recipe.output.glaze)
            .ok_or(CatalogValidationError::MissingOrDuplicateCombination)?;
        if std::mem::replace(&mut seen[form_index][clay_index][glaze_index], true) {
            return Err(CatalogValidationError::MissingOrDuplicateCombination);
        }
    }
    if seen
        .iter()
        .flatten()
        .flatten()
        .any(|combination| !combination)
    {
        return Err(CatalogValidationError::MissingOrDuplicateCombination);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_complete_and_recipes_match_their_inputs_and_outputs() {
        assert_eq!(CERAMIC_RECIPES.len(), 3 * 3 * 4);
        assert_eq!(validate_catalog(), Ok(()));
        for form in CeramicForm::ALL {
            for clay in ClayMaterial::ALL {
                for glaze in Glaze::ALL {
                    let recipe = recipe_for(form, clay, glaze).expect("catalog combination exists");
                    assert_eq!(recipe.output.form, form);
                    assert_eq!(recipe.output.clay, clay);
                    assert_eq!(recipe.output.glaze, glaze);
                    assert_eq!(recipe.output.state, ProcessingState::Greenware);
                    assert_eq!(recipe.input.resource, clay.resource());
                    assert_eq!(recipe.input.quantity, 3);
                }
            }
        }
    }

    #[test]
    fn ceramic_keeps_its_properties_while_advancing_through_legal_stages() {
        let mut item = recipe_for(CeramicForm::Vase, ClayMaterial::Red, Glaze::Blue)
            .unwrap()
            .output
            .instantiate(CeramicObjectId(42));
        assert_eq!(item.transition_to(ProcessingState::Dry), Ok(()));
        assert_eq!(item.transition_to(ProcessingState::Fired), Ok(()));
        assert_eq!(item.id, CeramicObjectId(42));
        assert_eq!(item.form(), CeramicForm::Vase);
        assert_eq!(item.clay(), ClayMaterial::Red);
        assert_eq!(item.glaze(), Glaze::Blue);
        assert_eq!(item.state(), ProcessingState::Fired);
    }

    #[test]
    fn skipped_repeated_reversed_and_post_firing_transitions_are_rejected() {
        let mut item = recipe_for(CeramicForm::Cup, ClayMaterial::Common, Glaze::None)
            .unwrap()
            .output
            .instantiate(CeramicObjectId(1));
        assert_eq!(
            item.transition_to(ProcessingState::Fired),
            Err(InvalidStageTransition {
                from: ProcessingState::Greenware,
                to: ProcessingState::Fired,
            })
        );
        item.transition_to(ProcessingState::Dry).unwrap();
        assert!(item.transition_to(ProcessingState::Dry).is_err());
        item.transition_to(ProcessingState::Fired).unwrap();
        assert!(item.transition_to(ProcessingState::Greenware).is_err());
        assert!(item.transition_to(ProcessingState::Dry).is_err());
    }
}
