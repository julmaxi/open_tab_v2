import { useContext } from "react";
import { useView } from "../../View";
import { useSelect } from "downshift";
import { TournamentContext } from "../../TournamentContext";
import { useSpringRef, useSpring, animated } from "react-spring";

export function VenueSelector(props) {
    let tournamentId = useContext(TournamentContext).uuid;

    let venues = useView({ type: "Venues", tournament_uuid: tournamentId }, { venues: [] });

    let selectedItem = props.venue ? venues.venues.find((v) => v.uuid === props.venue.uuid) : null;

    const {
        isOpen,
        getToggleButtonProps,
        getMenuProps,
        highlightedIndex,
        closeMenu,
        getItemProps,
    } = useSelect({
        items: venues.venues,
        itemToString: item => (item ? item.name : ""),
        selectedItem: selectedItem || null,
    });
    const springRef = useSpringRef()

    let style = useSpring({
        from: { height: 0 },
        to: {
            height: isOpen ? 240 : 0
        },
    });

    return <div className="inline">
        <button type="button" {...getToggleButtonProps()}>
            {selectedItem ? selectedItem.name : "<No Venue>"}
        </button>
        <div className="w-0 h-0 relative z-40">
            <animated.div className="w-72 bg-white mt-1 shadow-md overflow-auto p-0 h-8" style={style}>
                <ul {...getMenuProps()} className="w-full" >
                    {isOpen &&

                        venues.venues.map((item, index) => (
                            <li key={item.name} {...getItemProps({ item, index })} onClick={() => {
                                props.onVenueChange(item);
                                closeMenu();
                            }}>
                                {item.name}
                            </li>
                        ))
                    }
                </ul>
            </animated.div>
        </div>
    </div>
}
