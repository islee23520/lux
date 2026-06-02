pub use crate::lux_game_context_types::{
    CameraState, CapabilityBlocker, ColliderSnapshot, ComponentPropertySnapshot,
    EngineObservationCapability, GameContextEngine, GameContextObservation, GameContextRefs,
    ObservationSources, RectTransformSnapshot, SceneHierarchyNode, TransformSnapshot,
    UiCoordinateState,
};

impl GameContextObservation {
    pub fn unity(refs: GameContextRefs, sources: ObservationSources) -> Self {
        Self::empty(
            GameContextEngine::Unity,
            EngineObservationCapability::Supported,
            refs,
            sources,
        )
    }

    pub fn refs_are_lux_ssot(&self) -> bool {
        self.refs.spec_ref.starts_with(".lux/specs/")
            && self.refs.run_evidence_ref.starts_with(".lux/evidence/")
            && self
                .refs
                .ticket_ref
                .as_ref()
                .is_none_or(|ticket_ref| ticket_ref.starts_with(".lux/tickets/"))
    }

    fn empty(
        engine: GameContextEngine,
        capability_status: EngineObservationCapability,
        refs: GameContextRefs,
        sources: ObservationSources,
    ) -> Self {
        Self {
            schema_version: 1,
            engine,
            capability_status,
            refs,
            sources,
            scene_hierarchy: Vec::new(),
            selected_object_path: None,
            components: Vec::new(),
            transforms: Vec::new(),
            rect_transforms: Vec::new(),
            colliders: Vec::new(),
            camera_state: None,
            ui_coordinates: Vec::new(),
            console_logs: Vec::new(),
            compile_logs: Vec::new(),
            playmode_state: None,
            input_trace_refs: Vec::new(),
            screenshot_refs: Vec::new(),
            vision_annotations: Vec::new(),
            capability_blockers: Vec::new(),
        }
    }

    pub fn with_scene_node(mut self, node: SceneHierarchyNode) -> Self {
        self.scene_hierarchy.push(node);
        self
    }

    pub fn with_selected_object(mut self, path: impl Into<String>) -> Self {
        self.selected_object_path = Some(path.into());
        self
    }

    pub fn with_component(mut self, component: ComponentPropertySnapshot) -> Self {
        self.components.push(component);
        self
    }

    pub fn with_transform(mut self, transform: TransformSnapshot) -> Self {
        self.transforms.push(transform);
        self
    }

    pub fn with_rect_transform(mut self, rect_transform: RectTransformSnapshot) -> Self {
        self.rect_transforms.push(rect_transform);
        self
    }

    pub fn with_collider(
        mut self,
        game_object_path: impl Into<String>,
        collider_type: impl Into<String>,
    ) -> Self {
        self.colliders.push(ColliderSnapshot {
            game_object_path: game_object_path.into(),
            collider_type: collider_type.into(),
        });
        self
    }

    pub fn with_camera_state(
        mut self,
        name: impl Into<String>,
        screen_size_xy: [f64; 2],
        world_position_xyz: [f64; 3],
    ) -> Self {
        self.camera_state = Some(CameraState {
            name: name.into(),
            screen_size_xy,
            world_position_xyz,
        });
        self
    }

    pub fn with_ui_coordinate(
        mut self,
        element_path: impl Into<String>,
        screen_position_xy: [f64; 2],
    ) -> Self {
        self.ui_coordinates.push(UiCoordinateState {
            element_path: element_path.into(),
            screen_position_xy,
        });
        self
    }

    pub fn with_console_log(mut self, log: impl Into<String>) -> Self {
        self.console_logs.push(log.into());
        self
    }

    pub fn with_compile_log(mut self, log: impl Into<String>) -> Self {
        self.compile_logs.push(log.into());
        self
    }

    pub fn with_playmode_state(mut self, state: impl Into<String>) -> Self {
        self.playmode_state = Some(state.into());
        self
    }

    pub fn with_input_trace(mut self, evidence_ref: impl Into<String>) -> Self {
        self.input_trace_refs.push(evidence_ref.into());
        self
    }

    pub fn with_screenshot_ref(mut self, evidence_ref: impl Into<String>) -> Self {
        self.screenshot_refs.push(evidence_ref.into());
        self
    }

    pub fn with_vision_annotation(mut self, annotation: impl Into<String>) -> Self {
        self.vision_annotations.push(annotation.into());
        self
    }
}

pub fn unsupported_engine_context_blocker(
    engine: GameContextEngine,
    refs: GameContextRefs,
    reason: impl Into<String>,
) -> GameContextObservation {
    let reason = reason.into();
    let evidence_ref = refs.run_evidence_ref.clone();
    let mut observation = GameContextObservation::empty(
        engine,
        EngineObservationCapability::Unsupported,
        refs,
        unsupported_sources(),
    );
    observation.capability_blockers.push(CapabilityBlocker {
        engine,
        reason,
        evidence_ref,
    });
    observation
}

fn unsupported_sources() -> ObservationSources {
    ObservationSources {
        scene_hierarchy: "capability_blocker".to_string(),
        selected_object: "capability_blocker".to_string(),
        component_properties: "capability_blocker".to_string(),
        transform: "capability_blocker".to_string(),
        rect_transform: "capability_blocker".to_string(),
        collider: "capability_blocker".to_string(),
        camera_state: "capability_blocker".to_string(),
        ui_coordinates: "capability_blocker".to_string(),
        console_logs: "capability_blocker".to_string(),
        compile_logs: "capability_blocker".to_string(),
        playmode_state: "capability_blocker".to_string(),
        input_trace: "capability_blocker".to_string(),
        screenshot_refs: "capability_blocker".to_string(),
        vision_annotations: "capability_blocker".to_string(),
    }
}
