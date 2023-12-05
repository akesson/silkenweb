use std::{cell::RefCell, collections::HashMap};

use silkenweb_base::document;
use wasm_bindgen::UnwrapThrowExt;

use super::{
    head_inner_html, unmount_head, wet_insert_mounted, wet_unmount, Document, DocumentHead,
    HeadNotFound,
};
use crate::{
    document::MountedChildVecMap,
    dom::{self, Wet},
    mount_point,
    node::element::{
        child_vec::{ChildVec, ChildVecHandle, ParentShared},
        Const, GenericElement,
    },
};

impl Document for Wet {
    type MountInHeadOutput = ();
    type MountOutput = ();

    fn mount(id: &str, element: impl Into<GenericElement<Self, Const>>) -> Self::MountOutput {
        let element = element.into();

        mount_point(id)
            .replace_with_with_node_1(&element.dom_element())
            .unwrap_throw();
        wet_insert_mounted(id, element);
    }

    fn mount_in_head(
        id: &str,
        head: DocumentHead<Self>,
    ) -> Result<Self::MountInHeadOutput, HeadNotFound> {
        let head_elem = <Wet as dom::private::Dom>::Element::from_element(
            document::head().ok_or(HeadNotFound)?.into(),
        );

        let child_vec = ChildVec::<Wet, ParentShared>::new(head_elem, 0);
        let child_vec_handle = child_vec.run(head.child_vec);

        insert_mounted_in_head(id, child_vec_handle);

        Ok(())
    }

    fn unmount_all() {
        wet_unmount();
        unmount_head(&MOUNTED_IN_HEAD);
    }

    fn head_inner_html() -> String {
        head_inner_html(&MOUNTED_IN_HEAD)
    }
}

fn insert_mounted_in_head(id: &str, child_vec: ChildVecHandle<Wet, ParentShared>) {
    let existing =
        MOUNTED_IN_HEAD.with(|mounted| mounted.borrow_mut().insert(id.to_string(), child_vec));

    assert!(
        existing.is_none(),
        "Attempt to insert duplicate id ({id}) into head"
    );
}

thread_local! {
    static MOUNTED_IN_HEAD: MountedChildVecMap<Wet> = RefCell::new(HashMap::new());
}
