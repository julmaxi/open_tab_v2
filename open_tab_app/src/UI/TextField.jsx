export default function TextField(props) {
    const className = "border rounded p-1 w-full";
    if (props.area) {
        return (
            <textarea
                className={className}
                placeholder={props.placeholder}
                value={props.value}
                onChange={props.onChange}
            />
        );
    }
    else {
        return (
            <input
                type="text"
                className={className}
                placeholder={props.placeholder}
                value={props.value}
                onChange={props.onChange}
            />
        );
    }
    
}