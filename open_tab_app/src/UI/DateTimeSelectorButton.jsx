import React, { useState } from "react";
import { Popover } from "./Popover";

export function DateTimeSelectorButton({ buttonFactory: buttonFactory="button", buttonProps: buttonProps = [], label, onSetDate }) {
    let ButtonFactory = buttonFactory;
    let [isOpen, setIsOpen] = useState(false);
    let trigger = <ButtonFactory {...buttonProps} onClick={() => {
        setIsOpen(!isOpen);
    }}>{label}</ButtonFactory>;

    return <Popover trigger={trigger} isOpen={isOpen} onOpen={() => {
        setIsOpen(true);
    }} onClose={() => {
        setIsOpen(false);
    }}>
        <div className="">
            <div><label>In</label> <input onKeyDown={(event) => {
                if (event.key === "Enter") {
                    event.preventDefault();
                    let val = parseInt(event.target.value);
                    if (!isNaN(val)) {
                        let date = new Date();
                        date.setMinutes(date.getMinutes() + val);
                        onSetDate(date);
                    }
                    setIsOpen(false);
                }
            }} type="number" className="w-8"></input> Minutes</div>
            <button onClick={() => {
                setIsOpen(false);
                onSetDate(new Date());
            }}>Now</button>
        </div>
    </Popover>;
}
