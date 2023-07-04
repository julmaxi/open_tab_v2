import React, { useEffect, useState } from 'react';
import { useCombobox } from 'downshift';


function ComboBox({placeholder, ...props}) {
    const [items, setItems] = React.useState([])
    const {
      isOpen,
      getToggleButtonProps,
      getLabelProps,
      getMenuProps,
      getInputProps,
      highlightedIndex,
      getItemProps,
      selectedItem,
    } = useCombobox({
      onInputValueChange({inputValue}) {
        setItems(books.filter(getBooksFilter(inputValue)))
      },
      items,
      itemToString(item) {
        return item ? item.title : ''
      },
    })

    return (
      <div>
        <div className="w-72 flex flex-col gap-1">
          <div className="flex shadow-sm bg-white gap-0.5">
            <input
              placeholder={placeholder || ""}
              className="w-full p-1.5"
              {...getInputProps()}
            />
            <button
              aria-label="toggle menu"
              className="px-2"
              type="button"
              {...getToggleButtonProps()}
            >
              {isOpen ? <>&#8593;</> : <>&#8595;</>}
            </button>
          </div>
        </div>
        <ul
          className={`absolute w-72 bg-white mt-1 shadow-md max-h-80 overflow-scroll p-0 ${
            !(isOpen && items.length) && 'hidden'
          }`}
          {...getMenuProps()}
        >
          {isOpen &&
            items.map((item, index) => (
              <li
                className={cx(
                  highlightedIndex === index && 'bg-blue-300',
                  selectedItem === item && 'font-bold',
                  'py-2 px-3 shadow-sm flex flex-col',
                )}
                key={`${item.value}${index}`}
                {...getItemProps({item, index})}
              >
                <span>{item.title}</span>
                <span className="text-sm text-gray-700">{item.author}</span>
              </li>
            ))}
        </ul>
      </div>
    )
}

export default ComboBox;