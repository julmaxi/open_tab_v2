import Button from "./Button";

function RoundCard(props) {
    return <div className="bg-gray-200 rounded-lg p-2 m-2">
        {props.round.name}
    </div>
}

function RoundGroup(props) {
    return <div className="flex flex-col items-center">
        <div className="flex flex-row flex-wrap">
            {
                props.rounds.map((round) => 
                    <RoundCard round={round} key={round.uuid} />
                )
            }
        </div>
    </div>
}

function RoundBreakHierarchy(props) {
    return <div className="flex-col w-full h-full border-t">
        <RoundGroup rounds={props.rounds} />
        <div className="flex flex-row justify-center">
            <Button role="primary">Add Break…</Button>
        </div>
        <div className="flex flex-row">
            {
                props.breaks.map((roundBreak) => 
                    <div className="flex flex-1">
                        <RoundBreakHierarchy rounds={roundBreak.rounds} key={roundBreak.uuid} breaks={roundBreak.breaks || []} />
                    </div>
                )
            }
        </div>
    </div>
}

function RoundsOverview(props) {
    let roundsHierarchy = {
        "rounds": [
            {
                "uuid": "00000000-0000-0000-0000-000000000001",
                "name": "Round 1",
                "has_draw": false,
            },
            {
                "uuid": "00000000-0000-0000-0000-000000000002",
                "name": "Round 2",
                "has_draw": false,
            },
            {
                "uuid": "00000000-0000-0000-0000-000000000003",
                "name": "Round 3",
                "has_draw": false,
            },
        ],
        "breaks": [
            {
                "uuid": "00000000-0000-0000-0000-000000000001",
                "rounds": [
                    {
                        "uuid": "00000000-0000-0000-0000-000000000004",
                        "name": "Round 4",
                        "has_draw": false,
                    },
                    {
                        "uuid": "00000000-0000-0000-0000-000000000005",
                        "name": "Round 5",
                        "has_draw": false,
                    },        
                ],
                "breaks": [
                    {
                        "uuid": "00000000-0000-0000-0000-000000000006",
                        "rounds": [
                            {
                                "uuid": "00000000-0000-0000-0000-000000000001",
                                "name": "Semi-Finals",
                                "has_draw": false,
                            },
                        ],
                        "breaks": [
                            {
                                "rounds": [{
                                    "uuid": "00000000-0000-0000-0000-000000000001",
                                    "name": "Finals",
                                    "has_draw": false,
                                }]
                            }
                        ]
                    }
                ]
            },
            {
                "uuid": "00000000-0000-0000-0000-000000000002",
                "rounds": [
                    {
                        "uuid": "00000000-0000-0000-0000-000000000020",
                        "name": "DAF Final"
                    }
                ]
            }
        ]
    };

    return <div>
        <RoundBreakHierarchy rounds={roundsHierarchy.rounds} breaks={roundsHierarchy.breaks}/>
    </div>
}


export function RoundsEditorRoute(props) {
    return <div><RoundsOverview /></div>
}