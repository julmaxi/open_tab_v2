<script>
    import { enhance } from '$app/forms';
    import { parseDate } from '$lib/date.js';
    import { onMount } from 'svelte';
    import Timer from './Timer.svelte';
    import bell from '$lib/assets/bell.wav';
    import { env } from '$env/dynamic/public';

    export let data;

    const TEAM_POSITION_NAMES = {
        0: 'Eröffnungsrede',
        1: 'Ergänzungsrede',
        2: 'Schlussrede',
    };

    let enableControls = data.timingInfo.participant_may_control;

    /**
     * @param {{ role: string; is_response: any; position: number; }} speech
     */
    function getSpeechName(speech) {
        if (speech.role === 'non_aligned') {
            if (speech.is_response) {
                return `Anwort auf ${speech.position + 1}. Freie Rede`;
            }
            return `${speech.position + 1}. Freie Rede`
        }
        let teamName = speech.role === 'government' ? 'Regierung' : 'Opposition';

        // @ts-ignore
        return `${TEAM_POSITION_NAMES[speech.position]} der ${teamName}`
    }

    function computeTimePassed(speech, now) {
        return speech?.start ? (
            speech.end ?
            speech.end.getTime() - speech.start.getTime() - speech.pauseMilliseconds:
            now.getTime() - speech.start.getTime() - speech.pauseMilliseconds
        ):
        0;
    }

    /**
     * @param {{ [x: string]: any; }} speeches
     * @param {string | number} currIdx
     */
    function notifyBackendOfSpeechChange(speeches, currIdx) {
        let currSpeech = speeches[currIdx];
        let speech = null;
        if (currSpeech) {
            speech = {
                speech_position: currSpeech.position,
                speech_role: currSpeech.role,
                is_response: currSpeech.isResponse,
            };
        }
        fetch(
            `timer/notify`,
            {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({
                    speech
                }),
            }
        );
    }

    /**
     * @param {{ start: number; end: number; segments: { duration: number; }[]; }} speech
     * 
     * @returns {[number | null, number | null]}
     */
    function findCurrSegment(speech) {
        if (speech) {
            let passedTime = computeTimePassed(speech, new Date());
            let currSegment = 0;
            let currSegmentEnd = 0;
            for (let segment of speech.segments) {
                if (passedTime >= currSegmentEnd) {
                    currSegment += 1;
                    currSegmentEnd += segment.duration;
                }
            }
            return [currSegment, currSegmentEnd];
        }
        else {
            return [null, null]
        }
    }

    /**
     * @param {{ start?: any; end?: any; position: any; role: any; target_length?: any; is_response: any; segments?: any; pause_milliseconds?: any; }} speech
     */
    function mapSpeech(
        speech
    ) {
        return {
            start: speech.start ? parseDate(speech.start) : null,
            end: speech.end ? parseDate(speech.end) : null,
            position: speech.position,
            role: speech.role,
            targetLength: speech.target_length,
            name: getSpeechName(speech),
            isResponse: speech.is_response,
            segments: speech.segments.map(
                (/** @type {{ duration: number; }} */ segment) => {
                    return {
                        ...segment,
                        duration: segment.duration * 1000.0,
                    }
                }
            ),
            pauseMilliseconds: speech.pause_milliseconds
        }
    }

    let speeches = data.timingInfo.speeches.map(
        mapSpeech
    );
    let now = new Date();

    // @ts-ignore
    let currIdx = speeches.findIndex(speech => !speech.end);
    if (currIdx == -1) {
        currIdx = speeches.length;
    }
    $: currSpeech = currIdx < speeches.length ? speeches[currIdx] : null;

    /**
     * @type {number|null}
     * 
     */
    let currSegment = 0;
    /**
     * @type {number|null}
     * 
     */
    let currSegmentEnd = 0;

    $: currSpeechTimePassed = currSpeech?.start ? (
        currSpeech.end ?
            currSpeech.end.getTime() - currSpeech.start.getTime() - currSpeech.pauseMilliseconds:
            now.getTime() - currSpeech.start.getTime() - currSpeech.pauseMilliseconds
        ):
        0;

    //$: [currSegment, currSegmentEnd] = findCurrSegment(currSpeech);

    /**
     * @type {ReturnType<typeof setTimeout> | null}
     */
    let nextSegmentTimer = null;

    let scheduledRings = 0;

    function resetNextSegmentTimer(currSpeech, currSpeechEnd, currSpeechStart) {
        if (nextSegmentTimer !== null) {
            clearTimeout(nextSegmentTimer);
        }
        [currSegment, currSegmentEnd] = findCurrSegment(currSpeech);
        

        if (currSpeechEnd && currSpeechStart === null && currSegmentEnd !== null) {
            nextSegmentTimer = setTimeout(
                () => {
                    let bellElement = document.querySelector("#bell_audio");
                    scheduledRings = 2;
                    // @ts-ignore
                    bellElement.play().catch(
                        (err) => {
                            console.error(err);
                        }
                    );
                    resetNextSegmentTimer(currSpeech, currSpeechEnd, currSpeechStart);
                },
                currSegmentEnd - currSpeechTimePassed
            );
        }
    }


    $: resetNextSegmentTimer(currSpeech, currSpeech?.start, currSpeech?.end);


    /**
     * @type {AudioContext | null}
     */
    let audioContext = null;

    onMount(() => {
        const bellElement = document.querySelector("#bell_audio");
        bellElement.onended = () => {
            console.log(scheduledRings);
            if (scheduledRings > 0) {
                scheduledRings -= 1;
                bellElement.play().catch(
                    (err) => {
                        console.error(err);
                    }
                )
            }
        }

        const requestWakeLock = () => {navigator?.wakeLock?.request().then(
            (wakeLock) => {
                wakeLock.addEventListener('release', () => {
                    console.log('Wake Lock was released');
                });
            }
        )};


        const handleVisibilityChange = async () => {
            requestWakeLock();
        };
        document.addEventListener('visibilitychange', handleVisibilityChange);

        setInterval(
            () => {
                now = new Date();
            },
            1
        );

        let source = new EventSource(
            `${env.PUBLIC_API_URL}/api/notifications/participant/${data.participantId}`
        );
        source.addEventListener("timer", (event) => {
            let data = JSON.parse(event.data);

            let eventInfo = data.event;
            if (eventInfo.type == "ActiveSpeechUpdate") {
                if (!eventInfo.speech) {
                    currIdx = speeches.length;
                }
                else {
                    for (let idx = 0; idx < speeches.length; idx++) {
                        let speech = speeches[idx];
                        if (speech.position === eventInfo.speech.speech_position && speech.role === eventInfo.speech.speech_role && speech.isResponse === eventInfo.speech.is_response) {
                            currIdx = idx;
                            break
                        }
                    }
                }
            }
            else {
                for (let idx = 0; idx < speeches.length; idx++) {
                    let speech = speeches[idx];
                    if (speech.position === eventInfo.speech_position && speech.role === eventInfo.speech_role && speech.isResponse === eventInfo.is_response) {
                        speech.start = eventInfo.start ? parseDate(eventInfo.start) : null;
                        speech.end = eventInfo.end ? parseDate(eventInfo.end) : null;
                        speech.pauseMilliseconds = eventInfo.pause_milliseconds;
                        speeches = speeches;
                    }
                }
            }

        });

        source.onerror = (event) => {
            console.log(event);
        }
    });
</script>

<style>
    .timer-container {
        display: flex;
        flex-direction: column;
        align-items: center;
        height: calc(100vh - 3rem);
        padding: 0.5rem;
    }

    .padding {
        flex: 1;
        max-height: 250px;
    }

    .top-padding {
        height: calc(50% - 100px);
    }

    button {
        margin-top: 1rem;
        padding: 0.5rem;
        font-size: 1.5rem;
        text-align: center;
        width: 100%;
        max-width: 20rem;
        border-radius: 0.5rem;
        border-color: #ccc;
        border-width: 1px;
    }

    form {
        width: 100%;
        display: flex;
        flex-direction: column;
        align-items: center;
    }

    .info {
        font-style: italic;
    }
</style>

<audio id="bell_audio">
    <source src={bell} type="audio/wav">
</audio>


<div class="timer-container">
    <div class="top-padding"></div>

    {#if currSpeech}
        <span>
            {currSpeech.name}
        </span>

        <Timer targetTime={currSpeech.targetLength} timePassed={currSpeechTimePassed} />

        {#if enableControls }
            <form method="POST" action="?/setTime" use:enhance={
                (evt) => {
                    let time = new Date();
                    evt.data.append(
                        'time', time.toISOString()
                    );
                    
                    time = new Date(time.toISOString())

                    if (currSpeech.start) {
                        currSpeech.end = time;
                    }
                    else {
                        currSpeech.start = time;
                    }
                    currSpeech = currSpeech;
                }
            }>
                <input type="hidden" name="speechPosition" value={currSpeech.position} />
                <input type="hidden" name="speechRole" value={currSpeech.role} />
                <input type="hidden" name="isResponse" value={currSpeech.isResponse ? "true" : "false"} />
                <input type="hidden" name="key" value={currSpeech.start === null ? "start" : "end"} />

                {#if currSpeech.start === null || currSpeech.end === null}
                    <button type="submit" on:click={
                        () => {
                            if (!audioContext) {
                                audioContext = new window.AudioContext;
                                const bellElement = document.querySelector("#bell_audio");
                                // @ts-ignore
                                let track = audioContext.createMediaElementSource(bellElement);
                                track.connect(audioContext.destination);
                            }
                        }
                    }>{ currSpeech.start === null ? "Start" : "Stop" }</button>
                {/if}
            </form>
        {/if}
    {:else}
        <span>The debate is over</span>
    {/if}

    {#if enableControls && currSpeech && currSpeech.start !== null && currSpeech.end !== null}
    <button on:click={
        () => {
            currIdx += 1;
            notifyBackendOfSpeechChange(speeches, currIdx);
        }
    }>Next</button>
    {/if}

    {#if !enableControls}
        <p class="info">Only adjudicators can control the timer</p>
    {/if}

    <div class="padding"></div>
    

    {#if enableControls && currSpeech }
    <form method="POST" action="?/resume" use:enhance={
        (evt) => {
            let now = new Date();
            currSpeech.pauseMilliseconds = currSpeech.pauseMilliseconds + now.getTime() - currSpeech.end.getTime();
            currSpeech.end = null;

            evt.data.append(
                'resumeTime', now.toISOString()
            );
        }
    } style={
        currSpeech && currSpeech.end ? "" : "visibility: hidden;"
    }>
        <input type="hidden" name="speechPosition" value={currSpeech.position} />
        <input type="hidden" name="speechRole" value={currSpeech.role} />
        <input type="hidden" name="isResponse" value={currSpeech.isResponse ? "true" : "false"} />
        <input type="hidden" name="speechEnd" value={currSpeech.end} />
        <input type="hidden" name="previousPause" value={currSpeech.pauseMilliseconds} />
        
        <button type="submit">
            Resume
        </button>
    </form>
    {/if}

    {#if enableControls && currSpeech }
    <form method="POST" action="?/reset" use:enhance={
        () => {
            currSpeech.end = null;
            currSpeech.start = null;
            currSpeech.pauseMilliseconds = 0;
        }
    } style={
        currSpeech && currSpeech.end ? "" : "visibility: hidden;"
    }>
        <input type="hidden" name="speechPosition" value={currSpeech.position} />
        <input type="hidden" name="speechRole" value={currSpeech.role} />
        <input type="hidden" name="isResponse" value={currSpeech.isResponse ? "true" : "false"} />

        <button type="submit">
            Reset
        </button>
    </form>
    {/if}
    
    
    {#if enableControls}
        <button on:click={
            () => {
                currIdx -= 1;
                notifyBackendOfSpeechChange(speeches, currIdx);
            }
        } style={
            currIdx === 0 ? "visibility: hidden;" : ""
        }>
            Back
        </button>
    {/if}
</div>