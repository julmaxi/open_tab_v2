import * as React from "react";
import {
  useFloating,
  autoUpdate,
  offset,
  flip,
  shift,
  useClick,
  useFocus,
  useDismiss,
  useRole,
  useInteractions,
  useMergeRefs,
  FloatingPortal
} from "@floating-ui/react";


export function useTooltip({
  initialOpen = false,
  placement = "top",
  open: controlledOpen,
  onOpenChange: setControlledOpen
}) {
  const [uncontrolledOpen, setUncontrolledOpen] = React.useState(initialOpen);

  const open = controlledOpen ?? uncontrolledOpen;
  const setOpen = setControlledOpen ?? setUncontrolledOpen;

  const data = useFloating({
    placement,
    open,
    onOpenChange: setOpen,
    whileElementsMounted: autoUpdate,
    middleware: [
      offset(5),
      flip({
        crossAxis: placement.includes("-"),
        fallbackAxisSideDirection: "start",
        padding: 5
      }),
      shift({ padding: 5 })
    ]
  });

  const context = data.context;

  const dismiss = useDismiss(context);
  const role = useRole(context, { role: "tooltip" });
  const click = useClick(context);

  const interactions = useInteractions([dismiss, role, click]);

  setTimeout(() => {
    setOpen(false);
  }, 1000);

  return React.useMemo(
    () => ({
      open,
      setOpen,
      ...interactions,
      ...data
    }),
    [open, setOpen, interactions, data]
  );
}


const TooltipContext = React.createContext(null);

export const useTooltipContext = () => {
  const context = React.useContext(TooltipContext);

  if (context == null) {
    throw new Error("Tooltip components must be wrapped in <Tooltip />");
  }

  return context;
};

export function Tooltip({
  children,
  ...options
}) {
  // This can accept any props as options, e.g. `placement`,
  // or other positioning options.
  const tooltip = useTooltip(options);
  return (
    <TooltipContext.Provider value={tooltip}>
      {children}
    </TooltipContext.Provider>
  );
}

export const TooltipTrigger = React.forwardRef(function TooltipTrigger({ children, asChild = false, ...props }, propRef) {
  const context = useTooltipContext();
  const childrenRef = children.ref;
  const ref = useMergeRefs([context.refs.setReference, propRef, childrenRef]);

  let refProps = context.getReferenceProps({
    ref,
    ...props,
    ...children.props,
    "data-state": context.open ? "open" : "closed"
  });

  //Silly little hack to work around the inflexibility in floating UI.
  //Makes sure the tooltip is shown when the user attempts to start
  //dragging an element, which is needed for the drag and drop functionality
  //to indicate drag drop is disabled.
  refProps.onPointerDown = refProps.onClick

  // `asChild` allows the user to pass any element as the anchor
  if (asChild && React.isValidElement(children)) {
    return React.cloneElement(
      children,
      refProps
    );
  }

  return (
    <button
      ref={ref}
      // The user can style the trigger based on the state
      data-state={context.open ? "open" : "closed"}
      {...context.getReferenceProps(props)}
    >
      {children}
    </button>
  );
});

export const TooltipContent = React.forwardRef(function TooltipContent({ style, ...props }, propRef) {
  const context = useTooltipContext();
  const ref = useMergeRefs([context.refs.setFloating, propRef]);

  if (!context.open) return null;

  return (
    <FloatingPortal>
      <div
        ref={ref}
        style={{
          ...context.floatingStyles,
          ...style
        }}
        {...context.getFloatingProps(props)}
      />
    </FloatingPortal>
  );
});
