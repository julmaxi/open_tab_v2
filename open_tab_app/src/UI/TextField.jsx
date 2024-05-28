export default function TextField(props) {
  return (
    <input
      type="text"
      className="border rounded p-1 w-full"
      placeholder={props.placeholder}
      value={props.value}
      onChange={props.onChange}
    />
  );
}