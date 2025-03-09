import { useRef, cloneElement } from "react";
import {
    useFloating,
    autoUpdate,
    offset,
    flip,
    shift,
    arrow,
    useDismiss,
    useRole,
    useClick,
    useHover,
    useInteractions,
    FloatingFocusManager,
    useId,
    FloatingArrow,
    FloatingPortal,
    safePolygon
} from '@floating-ui/react';

export function Popover(props) {
    const arrowRef = useRef(null);

    const { refs, floatingStyles, context } = useFloating({
        open: props.isOpen,
        onOpenChange: (open) => {
            if (open) {
                props.onOpen();
            }
            else {
                props.onClose();
            }
        },
        placement: "right",
        middleware: [
            offset(10),
            flip({ fallbackAxisSideDirection: "end" }),
            shift(),
            arrow({
                element: arrowRef,
            }),
        ],
        whileElementsMounted: autoUpdate
    });

    const click = useClick(context);
    const hover = useHover(context, {
        enabled: props.hover === true,
        handleClose: safePolygon({
            requireIntent: false,
        }),        
    });
    const dismiss = useDismiss(context);
    const role = useRole(context);

    const { getReferenceProps, getFloatingProps } = useInteractions([
        click,
        dismiss,
        role,
        hover
    ]);

    const headingId = useId();

    return (
        <>
            {cloneElement(
                props.trigger,
                getReferenceProps({
                    ref: refs.setReference,
                    ...props.trigger.props,
                    "data-state": context.open ? "open" : "closed"
                }))}

            {props.isOpen && (
            <FloatingPortal>
                <FloatingFocusManager context={context} modal={false}>
                    <div
                        className="bg-white border rounded p-2 absolute z-10"
                        ref={refs.setFloating}
                        style={floatingStyles}
                        aria-labelledby={headingId}
                        {...getFloatingProps()}
                    >
                        <FloatingArrow ref={arrowRef} context={context} />
                        {props.children}
                    </div>
                </FloatingFocusManager>
            </FloatingPortal>
            )}
        </>
    );
}
