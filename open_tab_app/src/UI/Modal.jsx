import React, { useCallback, useContext } from "react";
import { createPortal } from 'react-dom';

import { useState, useMemo } from "react";

export default function ModalOverlay(props) {

    let body = props.open ? <div className="fixed top-0 left-0 z-50 w-full overflow-x-hidden overflow-y-hidden inset-0 h-full">
    <div tabIndex={-1} className={"absolute top-0 left-0 w-full h-full bg-opacity-50 bg-black"} onClick={() => {
        if (props.closeOnOverlayClick) {props.onAbort()};
    }} />                
    <div className="w-full h-full grid place-items-center bg-none">
        <DialogWindow windowClassName={props.windowClassName || ""}>
            {props.children}
        </DialogWindow>
    </div>
</div> : <div className="hidden" />;

    return createPortal(body, document.body);
}

function DialogWindow(props) {
    return <div className={`z-10 bg-white p-8 ${props.windowClassName}`}>
        {props.children}
    </div>
}
