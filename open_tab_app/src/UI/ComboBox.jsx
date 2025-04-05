import React, { useEffect, useState, useMemo } from 'react';
import { useCombobox } from 'downshift';

function makeClassname(...args) {
    return args.filter((arg) => arg !== undefined).join(" ");
}

function ComboBox({ placeholder, items, onSelect, allowCreate, ignoredItemNames, value = null, ...props }) {
    const [filter, setFilter] = React.useState(null);

    const shownItems = useMemo(() => {
        let availableItems = items.filter((item) => {
            return !(ignoredItemNames || []).includes(item.name);
        });
        if (filter === null) {
            return availableItems;
        }
        else {
            return availableItems.filter((item) => {
                return item.name.toLowerCase().includes(filter.toLowerCase());
            });
        }
    }, [items, filter, ignoredItemNames]);

    const {
        isOpen,
        getToggleButtonProps,
        getLabelProps,
        getMenuProps,
        getInputProps,
        highlightedIndex,
        getItemProps,
        selectedItem,
        inputValue,
        reset
    } = useCombobox({
        onInputValueChange({ inputValue }) {
            setFilter(inputValue);
        },
        selectedItem: null,
        items: shownItems,
        itemToString(item) {
            return item ? item.name : ''
        },
        onSelectedItemChange({ selectedItem }) {
            onSelect(selectedItem, false)
        },
        stateReducer: (state, actionAndChanges) => {
            const { changes, type } = actionAndChanges
            switch (type) {
                case useCombobox.stateChangeTypes.InputKeyDownEnter:
                case useCombobox.stateChangeTypes.ItemClick:
                    return {
                        ...changes,
                        isOpen: false,
                        highlightedIndex: state.highlightedIndex,
                        inputValue: '',
                    }
                case useCombobox.stateChangeTypes.InputBlur:
                    return {
                        ...changes,
                        inputValue: '',
                    }
                default:
                    return changes
            }
        },
    });
    let toCreateExists = allowCreate && (shownItems.some((item) => item.name.toLowerCase() == inputValue.toLowerCase())
    || (ignoredItemNames || []).includes(inputValue));

    let isReallyOpen = isOpen || !!inputValue;

    let inputProps = {...getInputProps()};
    if (value && !isOpen) {
        inputProps.value = value;
    }

    return (
        <div>
            <div className="w-72 flex flex-col gap-1">
                <div className="flex shadow-sm bg-white gap-0.5">
                    <input
                        placeholder={placeholder || ""}
                        className="w-full p-1.5 border rounded"
                        {...inputProps}
                    />
                    <button
                        aria-label="toggle menu"
                        className="px-2"
                        type="button"
                        {...getToggleButtonProps()}
                    >
                        {isReallyOpen ? <>&#8593;</> : <>&#8595;</>}
                    </button>
                </div>
            </div>
            <ul
                className={`w-72 bg-white mt-1 shadow-md max-h-80 overflow-auto p-0 z-10 ${!(isReallyOpen) && 'hidden'
                    }`}
                {...getMenuProps()}
            >
                {isReallyOpen &&
                    <>
                        {
                            shownItems.map((item, index) => (
                                <li
                                    className={makeClassname(
                                        highlightedIndex === index && 'bg-blue-300',
                                        selectedItem === item && 'font-bold',
                                        'py-2 px-3 shadow-sm flex flex-col',
                                    )}
                                    key={`${item.value}${index}`}
                                    {...getItemProps({ item, index })}
                                >
                                    <span>{item.name}</span>
                                </li>
                            ))
                        }
                        {
                            allowCreate && !toCreateExists &&
                            <li onClick={() => {
                                reset();
                                onSelect(inputValue, true);
                            }} className={
                                makeClassname(
                                    'py-2 px-3 shadow-sm flex flex-col',
                                )
                            }>
                                Create {inputValue}
                            </li>
                        }
                    </>
                }
            </ul>
        </div>
    )
}

export default ComboBox;