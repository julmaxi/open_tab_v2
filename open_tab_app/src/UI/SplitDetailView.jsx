import React from "react";
import { useState } from "react";

export function SplitDetailView({ children, initialDetailWidth=400 }) {
    let [width, setWidth] = useState(initialDetailWidth);

    let numChildren = 0;
    React.Children.forEach(children, (c) => {
        if (c) {
            numChildren++;
        }
    });

    if (numChildren > 2) {
        throw new Error("ResizableView must not have more than two children");
    }

    return <div className="flex w-full h-full">
        <div className="flex-1">
            {children[0]}
        </div>
        {numChildren > 1 ? <div style={{ "width": width, "minWidth": "200px" }} className="relative h-full overflow-scroll border-l-2">
            <DragHandle onDragDelta={(delta) => {
                let newWidth = width - delta;
                if (newWidth < 200) {
                    newWidth = 200;
                }
                setWidth(newWidth);
            }} />

            {children[1]}
        </div>
            : []}
    </div>;
}
function DragHandle({ onDragDelta }) {
    return <div className="absolute top-0 left-0 h-full w-1 cursor-ew-resize" onMouseDown={(e) => {
        let startX = e.clientX;
        let onMouseMove = (e) => {
            onDragDelta(e.clientX - startX);
            e.preventDefault();
        };
        let onMouseUp = () => {
            window.removeEventListener("mousemove", onMouseMove);
            window.removeEventListener("mouseup", onMouseUp);
        };
        window.addEventListener("mousemove", onMouseMove);
        window.addEventListener("mouseup", onMouseUp);
    }} />;
}
