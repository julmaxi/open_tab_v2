


const ExponentialStepper = ({ value, onChange, ...props }) => {
    return (
        <div className="flex flex-row">
        <button
            onClick={() => value > 2 ? onChange(Math.ceil(value / 2)): null}
            className="bg-gray-200 hover:bg-gray-300 text-gray-600 font-bold py-2 px-4 rounded-l"
        >
            &lt;
        </button>
        <input
            type="text"
            value={value}
            onChange={e => onChange(e.target.value)}
            className="bg-gray-200 text-gray-600 font-bold w-8 text-center"
        />
        <button
            onClick={() => onChange(value * 2)}
            className="bg-gray-200 hover:bg-gray-300 text-gray-600 font-bold py-2 px-4 rounded-r"
        >
            &gt;
        </button>
        </div>
    );
}; 

export default ExponentialStepper;
