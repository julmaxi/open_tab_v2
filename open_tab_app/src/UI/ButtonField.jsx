
const ButtonField = ({ label, onClick, placeholder }) => (
    <div className="w-full flex flex-row">
    <div className={`text-center border rounded align-middle rounded-r-none flex-grow ${label ? "font-medium text-sm" : "text-gray-400 text-xs"}`}>
        {label ? label : placeholder}
    </div>
    <button 
        onClick={onClick}
        className="bg-blue-500 text-white px-2 py-1 text-xs rounded-md rounded-l-none"
    >
        Select
    </button>
  </div>
);

export default ButtonField;