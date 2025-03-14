import React from "react";


export function HorizontalList(props) {
    return <div className="flex flex-row gap-x-1">
        {props.children}
    </div>;
}
