//! Help tab content (spec 20) — generated from web `helpContent.ts`; keys resolve in the `help` namespace unless prefixed.

use crate::icons::Icon;

pub struct HelpItem {
    pub label: &'static str,
    pub description: &'static str,
}

pub struct HelpSection {
    pub heading: &'static str,
    pub items: &'static [HelpItem],
}

pub struct HelpTab {
    pub id: &'static str,
    pub label: &'static str,
    pub icon: Icon,
    /// Empty for the About tab (custom content).
    pub sections: &'static [HelpSection],
}

pub const HELP_TABS: &[HelpTab] = &[
    HelpTab {
        id: "pin",
        label: "common:modes.pin",
        icon: Icon::Pin,
        sections: &[
            HelpSection {
                heading: "help:pin.drawingTools.heading",
                items: &[
                    HelpItem {
                        label: "help:pin.drawingTools.line.label",
                        description: "help:pin.drawingTools.line.description",
                    },
                    HelpItem {
                        label: "help:pin.drawingTools.arc.label",
                        description: "help:pin.drawingTools.arc.description",
                    },
                    HelpItem {
                        label: "help:pin.drawingTools.ellipse.label",
                        description: "help:pin.drawingTools.ellipse.description",
                    },
                    HelpItem {
                        label: "help:pin.drawingTools.circle.label",
                        description: "help:pin.drawingTools.circle.description",
                    },
                    HelpItem {
                        label: "help:pin.drawingTools.rect.label",
                        description: "help:pin.drawingTools.rect.description",
                    },
                    HelpItem {
                        label: "help:pin.drawingTools.square.label",
                        description: "help:pin.drawingTools.square.description",
                    },
                    HelpItem {
                        label: "help:pin.drawingTools.freehand.label",
                        description: "help:pin.drawingTools.freehand.description",
                    },
                    HelpItem {
                        label: "help:pin.drawingTools.path.label",
                        description: "help:pin.drawingTools.path.description",
                    },
                    HelpItem {
                        label: "help:pin.drawingTools.eraser.label",
                        description: "help:pin.drawingTools.eraser.description",
                    },
                    HelpItem {
                        label: "help:pin.drawingTools.pathEraser.label",
                        description: "help:pin.drawingTools.pathEraser.description",
                    },
                    HelpItem {
                        label: "help:pin.drawingTools.polygonStar.label",
                        description: "help:pin.drawingTools.polygonStar.description",
                    },
                ],
            },
            HelpSection {
                heading: "help:pin.pinProperties.heading",
                items: &[
                    HelpItem {
                        label: "help:pin.pinProperties.spacing.label",
                        description: "help:pin.pinProperties.spacing.description",
                    },
                    HelpItem {
                        label: "help:pin.pinProperties.actualGap.label",
                        description: "help:pin.pinProperties.actualGap.description",
                    },
                    HelpItem {
                        label: "help:pin.pinProperties.diameter.label",
                        description: "help:pin.pinProperties.diameter.description",
                    },
                    HelpItem {
                        label: "help:pin.pinProperties.colour.label",
                        description: "help:pin.pinProperties.colour.description",
                    },
                    HelpItem {
                        label: "help:pin.pinProperties.guideVisible.label",
                        description: "help:pin.pinProperties.guideVisible.description",
                    },
                ],
            },
            HelpSection {
                heading: "help:pin.symmetry.heading",
                items: &[
                    HelpItem {
                        label: "help:pin.symmetry.modes.label",
                        description: "help:pin.symmetry.modes.description",
                    },
                    HelpItem {
                        label: "help:pin.symmetry.interval.label",
                        description: "help:pin.symmetry.interval.description",
                    },
                    HelpItem {
                        label: "help:pin.symmetry.centre.label",
                        description: "help:pin.symmetry.centre.description",
                    },
                ],
            },
        ],
    },
    HelpTab {
        id: "edit",
        label: "common:modes.select",
        icon: Icon::MousePointer2,
        sections: &[
            HelpSection {
                heading: "help:edit.tools.heading",
                items: &[
                    HelpItem {
                        label: "help:edit.tools.select.label",
                        description: "help:edit.tools.select.description",
                    },
                    HelpItem {
                        label: "help:edit.tools.move.label",
                        description: "help:edit.tools.move.description",
                    },
                    HelpItem {
                        label: "help:edit.tools.rotation.label",
                        description: "help:edit.tools.rotation.description",
                    },
                    HelpItem {
                        label: "help:edit.tools.scale.label",
                        description: "help:edit.tools.scale.description",
                    },
                    HelpItem {
                        label: "help:edit.tools.merge.label",
                        description: "help:edit.tools.merge.description",
                    },
                ],
            },
            HelpSection {
                heading: "help:edit.selectionPanel.heading",
                items: &[
                    HelpItem {
                        label: "help:edit.selectionPanel.pinDistance.label",
                        description: "help:edit.selectionPanel.pinDistance.description",
                    },
                    HelpItem {
                        label: "help:edit.selectionPanel.geometryFields.label",
                        description: "help:edit.selectionPanel.geometryFields.description",
                    },
                    HelpItem {
                        label: "help:edit.selectionPanel.symmetry.label",
                        description: "help:edit.selectionPanel.symmetry.description",
                    },
                    HelpItem {
                        label: "help:edit.selectionPanel.deletePinPath.label",
                        description: "help:edit.selectionPanel.deletePinPath.description",
                    },
                ],
            },
        ],
    },
    HelpTab {
        id: "thread",
        label: "common:modes.thread",
        icon: Icon::Spline,
        sections: &[
            HelpSection {
                heading: "help:thread.tools.heading",
                items: &[
                    HelpItem {
                        label: "help:thread.tools.draw.label",
                        description: "help:thread.tools.draw.description",
                    },
                    HelpItem {
                        label: "help:thread.tools.zigzag.label",
                        description: "help:thread.tools.zigzag.description",
                    },
                    HelpItem {
                        label: "help:thread.tools.parabolic.label",
                        description: "help:thread.tools.parabolic.description",
                    },
                    HelpItem {
                        label: "help:thread.tools.radial.label",
                        description: "help:thread.tools.radial.description",
                    },
                    HelpItem {
                        label: "help:thread.tools.repeat.label",
                        description: "help:thread.tools.repeat.description",
                    },
                    HelpItem {
                        label: "help:thread.tools.eraser.label",
                        description: "help:thread.tools.eraser.description",
                    },
                    HelpItem {
                        label: "help:thread.tools.segment.label",
                        description: "help:thread.tools.segment.description",
                    },
                ],
            },
            HelpSection {
                heading: "help:thread.properties.heading",
                items: &[
                    HelpItem {
                        label: "help:thread.properties.colourCount.label",
                        description: "help:thread.properties.colourCount.description",
                    },
                    HelpItem {
                        label: "help:thread.properties.swatches.label",
                        description: "help:thread.properties.swatches.description",
                    },
                    HelpItem {
                        label: "help:thread.properties.width.label",
                        description: "help:thread.properties.width.description",
                    },
                    HelpItem {
                        label: "help:thread.properties.twistPitch.label",
                        description: "help:thread.properties.twistPitch.description",
                    },
                    HelpItem {
                        label: "help:thread.properties.pinStateLegend.label",
                        description: "help:thread.properties.pinStateLegend.description",
                    },
                ],
            },
        ],
    },
    HelpTab {
        id: "pan",
        label: "common:modes.pan",
        icon: Icon::Hand,
        sections: &[
            HelpSection {
                heading: "help:pan.canvas.heading",
                items: &[HelpItem {
                    label: "help:pan.canvas.leftClickDrag.label",
                    description: "help:pan.canvas.leftClickDrag.description",
                }],
            },
            HelpSection {
                heading: "help:pan.grid.heading",
                items: &[
                    HelpItem {
                        label: "help:pan.grid.gridToggle.label",
                        description: "help:pan.grid.gridToggle.description",
                    },
                    HelpItem {
                        label: "help:pan.grid.snapToggle.label",
                        description: "help:pan.grid.snapToggle.description",
                    },
                    HelpItem {
                        label: "help:pan.grid.gapXY.label",
                        description: "help:pan.grid.gapXY.description",
                    },
                    HelpItem {
                        label: "help:pan.grid.gridColour.label",
                        description: "help:pan.grid.gridColour.description",
                    },
                    HelpItem {
                        label: "help:pan.grid.gridOpacity.label",
                        description: "help:pan.grid.gridOpacity.description",
                    },
                ],
            },
            HelpSection {
                heading: "help:pan.zoom.heading",
                items: &[
                    HelpItem {
                        label: "help:pan.zoom.zoomInOut.label",
                        description: "help:pan.zoom.zoomInOut.description",
                    },
                    HelpItem {
                        label: "help:pan.zoom.fit.label",
                        description: "help:pan.zoom.fit.description",
                    },
                ],
            },
        ],
    },
    HelpTab {
        id: "play",
        label: "common:modes.play",
        icon: Icon::Play,
        sections: &[
            HelpSection {
                heading: "help:play.transport.heading",
                items: &[
                    HelpItem {
                        label: "help:play.transport.firstPrevNextLast.label",
                        description: "help:play.transport.firstPrevNextLast.description",
                    },
                    HelpItem {
                        label: "help:play.transport.playPause.label",
                        description: "help:play.transport.playPause.description",
                    },
                    HelpItem {
                        label: "help:play.transport.frame.label",
                        description: "help:play.transport.frame.description",
                    },
                    HelpItem {
                        label: "help:play.transport.total.label",
                        description: "help:play.transport.total.description",
                    },
                    HelpItem {
                        label: "help:play.transport.interval.label",
                        description: "help:play.transport.interval.description",
                    },
                ],
            },
            HelpSection {
                heading: "help:play.export.heading",
                items: &[HelpItem {
                    label: "help:play.export.exportToVideo.label",
                    description: "help:play.export.exportToVideo.description",
                }],
            },
        ],
    },
    HelpTab {
        id: "layers",
        label: "help:tabs.layers",
        icon: Icon::Layers,
        sections: &[
            HelpSection {
                heading: "help:layers.tabSwitchSection.heading",
                items: &[HelpItem {
                    label: "help:layers.tabSwitchSection.tabSwitch.label",
                    description: "help:layers.tabSwitchSection.tabSwitch.description",
                }],
            },
            HelpSection {
                heading: "help:layers.row.heading",
                items: &[
                    HelpItem {
                        label: "help:layers.row.eyeIcon.label",
                        description: "help:layers.row.eyeIcon.description",
                    },
                    HelpItem {
                        label: "help:layers.row.lockIcon.label",
                        description: "help:layers.row.lockIcon.description",
                    },
                    HelpItem {
                        label: "help:layers.row.nameRename.label",
                        description: "help:layers.row.nameRename.description",
                    },
                    HelpItem {
                        label: "help:layers.row.clickRow.label",
                        description: "help:layers.row.clickRow.description",
                    },
                ],
            },
            HelpSection {
                heading: "help:layers.actions.heading",
                items: &[
                    HelpItem {
                        label: "help:layers.actions.newLayer.label",
                        description: "help:layers.actions.newLayer.description",
                    },
                    HelpItem {
                        label: "help:layers.actions.duplicate.label",
                        description: "help:layers.actions.duplicate.description",
                    },
                    HelpItem {
                        label: "help:layers.actions.merge.label",
                        description: "help:layers.actions.merge.description",
                    },
                    HelpItem {
                        label: "help:layers.actions.moveLayer.label",
                        description: "help:layers.actions.moveLayer.description",
                    },
                    HelpItem {
                        label: "help:layers.actions.delete.label",
                        description: "help:layers.actions.delete.description",
                    },
                ],
            },
        ],
    },
    HelpTab {
        id: "keyboard-mouse",
        label: "help:tabs.keyboardMouse",
        icon: Icon::Keyboard,
        sections: &[
            HelpSection {
                heading: "help:keyboardMouse.keyboard.heading",
                items: &[
                    HelpItem {
                        label: "help:keyboardMouse.keyboard.undo.label",
                        description: "help:keyboardMouse.keyboard.undo.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.keyboard.redo.label",
                        description: "help:keyboardMouse.keyboard.redo.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.keyboard.escape.label",
                        description: "help:keyboardMouse.keyboard.escape.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.keyboard.arrowLeft.label",
                        description: "help:keyboardMouse.keyboard.arrowLeft.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.keyboard.arrowRight.label",
                        description: "help:keyboardMouse.keyboard.arrowRight.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.keyboard.arrowUpDown.label",
                        description: "help:keyboardMouse.keyboard.arrowUpDown.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.keyboard.shift.label",
                        description: "help:keyboardMouse.keyboard.shift.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.keyboard.holdArrow.label",
                        description: "help:keyboardMouse.keyboard.holdArrow.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.keyboard.alt.label",
                        description: "help:keyboardMouse.keyboard.alt.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.keyboard.centerSnap.label",
                        description: "help:keyboardMouse.keyboard.centerSnap.description",
                    },
                ],
            },
            HelpSection {
                heading: "help:keyboardMouse.mouse.heading",
                items: &[
                    HelpItem {
                        label: "help:keyboardMouse.mouse.leftButton.label",
                        description: "help:keyboardMouse.mouse.leftButton.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.mouse.rightClick.label",
                        description: "help:keyboardMouse.mouse.rightClick.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.mouse.doubleClick.label",
                        description: "help:keyboardMouse.mouse.doubleClick.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.mouse.leftDrag.label",
                        description: "help:keyboardMouse.mouse.leftDrag.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.mouse.wheel.label",
                        description: "help:keyboardMouse.mouse.wheel.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.mouse.middleDrag.label",
                        description: "help:keyboardMouse.mouse.middleDrag.description",
                    },
                ],
            },
            HelpSection {
                heading: "help:keyboardMouse.shortcuts.general.heading",
                items: &[
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.general.save.label",
                        description: "help:keyboardMouse.shortcuts.general.save.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.general.open.label",
                        description: "help:keyboardMouse.shortcuts.general.open.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.general.new.label",
                        description: "help:keyboardMouse.shortcuts.general.new.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.general.help.label",
                        description: "help:keyboardMouse.shortcuts.general.help.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.general.zoomIn.label",
                        description: "help:keyboardMouse.shortcuts.general.zoomIn.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.general.zoomOut.label",
                        description: "help:keyboardMouse.shortcuts.general.zoomOut.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.general.fit.label",
                        description: "help:keyboardMouse.shortcuts.general.fit.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.general.grid.label",
                        description: "help:keyboardMouse.shortcuts.general.grid.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.general.snap.label",
                        description: "help:keyboardMouse.shortcuts.general.snap.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.general.delete.label",
                        description: "help:keyboardMouse.shortcuts.general.delete.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.general.copy.label",
                        description: "help:keyboardMouse.shortcuts.general.copy.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.general.paste.label",
                        description: "help:keyboardMouse.shortcuts.general.paste.description",
                    },
                ],
            },
            HelpSection {
                heading: "help:keyboardMouse.shortcuts.tabs.heading",
                items: &[HelpItem {
                    label: "help:keyboardMouse.shortcuts.tabs.tabSwitch.label",
                    description: "help:keyboardMouse.shortcuts.tabs.tabSwitch.description",
                }],
            },
            HelpSection {
                heading: "help:keyboardMouse.shortcuts.edit.heading",
                items: &[
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.edit.granularity.label",
                        description: "help:keyboardMouse.shortcuts.edit.granularity.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.edit.select.label",
                        description: "help:keyboardMouse.shortcuts.edit.select.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.edit.move.label",
                        description: "help:keyboardMouse.shortcuts.edit.move.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.edit.rotate.label",
                        description: "help:keyboardMouse.shortcuts.edit.rotate.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.edit.scale.label",
                        description: "help:keyboardMouse.shortcuts.edit.scale.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.edit.merge.label",
                        description: "help:keyboardMouse.shortcuts.edit.merge.description",
                    },
                ],
            },
            HelpSection {
                heading: "help:keyboardMouse.shortcuts.pin.heading",
                items: &[
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.pin.line.label",
                        description: "help:keyboardMouse.shortcuts.pin.line.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.pin.arc.label",
                        description: "help:keyboardMouse.shortcuts.pin.arc.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.pin.ellipse.label",
                        description: "help:keyboardMouse.shortcuts.pin.ellipse.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.pin.circle.label",
                        description: "help:keyboardMouse.shortcuts.pin.circle.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.pin.rectangle.label",
                        description: "help:keyboardMouse.shortcuts.pin.rectangle.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.pin.square.label",
                        description: "help:keyboardMouse.shortcuts.pin.square.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.pin.freehand.label",
                        description: "help:keyboardMouse.shortcuts.pin.freehand.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.pin.path.label",
                        description: "help:keyboardMouse.shortcuts.pin.path.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.pin.text.label",
                        description: "help:keyboardMouse.shortcuts.pin.text.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.pin.eraser.label",
                        description: "help:keyboardMouse.shortcuts.pin.eraser.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.pin.pathEraser.label",
                        description: "help:keyboardMouse.shortcuts.pin.pathEraser.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.pin.symmetryCycle.label",
                        description: "help:keyboardMouse.shortcuts.pin.symmetryCycle.description",
                    },
                ],
            },
            HelpSection {
                heading: "help:keyboardMouse.shortcuts.thread.heading",
                items: &[
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.thread.draw.label",
                        description: "help:keyboardMouse.shortcuts.thread.draw.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.thread.zigzag.label",
                        description: "help:keyboardMouse.shortcuts.thread.zigzag.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.thread.parabolic.label",
                        description: "help:keyboardMouse.shortcuts.thread.parabolic.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.thread.radial.label",
                        description: "help:keyboardMouse.shortcuts.thread.radial.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.thread.repeat.label",
                        description: "help:keyboardMouse.shortcuts.thread.repeat.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.thread.select.label",
                        description: "help:keyboardMouse.shortcuts.thread.select.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.thread.eraser.label",
                        description: "help:keyboardMouse.shortcuts.thread.eraser.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.thread.segmentEraser.label",
                        description: "help:keyboardMouse.shortcuts.thread.segmentEraser.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.thread.colourCount.label",
                        description: "help:keyboardMouse.shortcuts.thread.colourCount.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.thread.width.label",
                        description: "help:keyboardMouse.shortcuts.thread.width.description",
                    },
                ],
            },
            HelpSection {
                heading: "help:keyboardMouse.shortcuts.play.heading",
                items: &[
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.play.first.label",
                        description: "help:keyboardMouse.shortcuts.play.first.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.play.previous.label",
                        description: "help:keyboardMouse.shortcuts.play.previous.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.play.playPause.label",
                        description: "help:keyboardMouse.shortcuts.play.playPause.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.play.next.label",
                        description: "help:keyboardMouse.shortcuts.play.next.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.play.last.label",
                        description: "help:keyboardMouse.shortcuts.play.last.description",
                    },
                    HelpItem {
                        label: "help:keyboardMouse.shortcuts.play.export.label",
                        description: "help:keyboardMouse.shortcuts.play.export.description",
                    },
                ],
            },
        ],
    },
    HelpTab {
        id: "about",
        label: "help:tabs.about",
        icon: Icon::Info,
        sections: &[],
    },
];
