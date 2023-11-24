export default function Button(props) {
    let baseStyle = "ml-1 p-1 text-white rounded";

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

    if (props.disabled) {
        bgColor = "bg-gray-300";
    }

    return <button className={`${baseStyle} ${bgColor}`} disabled={props.disabled} onClick={props.onClick}>{props.children}</button>
}
