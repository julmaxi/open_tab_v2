import React from 'react';

const Button = React.forwardRef((props, ref) => {
    let baseStyle = "p-1 text-white rounded";

    let bgColor = "bg-gray-500";
    if (props.role == "primary") {
        bgColor = "bg-blue-500";
    }
    else if (props.role == "secondary") {
        bgColor = "bg-gray-500";
    }
    else if (props.role == "danger") {
        bgColor = "bg-red-500";
    }
    else if (props.role == "approve") {
        bgColor = "bg-green-500";
    }

    if (props.disabled) {
        bgColor = "bg-gray-300";
    }

    return <button ref={ref} className={`${baseStyle} ${bgColor} ${props.className}`} disabled={props.disabled} onClick={props.onClick}>{props.children}</button>
}
);

export default Button;