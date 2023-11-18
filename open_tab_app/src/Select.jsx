import React from 'react';

const Select = ({ label, options, value, onChange }) => {
    //Generate random name
    let name = Math.random().toString(36).substring(7);
  return (
    <div className="w-full max-w-xs mx-auto">
      {label && <label htmlFor={name} className="block text-sm font-medium text-gray-700">{label}</label>}
      <select
        id={name}
        value={value}
        onChange={onChange}
        className="mt-1 block w-full pl-3 pr-10 py-2 text-base border-gray-300 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm rounded-md"
      >
        {options.map(option => (
          <option key={option.value} value={option.value} disabled={
            !(option.selectable === undefined || option.selectable)
          }>
            {option.label}
          </option>
        ))}
      </select>
    </div>
  );
};

export default Select;