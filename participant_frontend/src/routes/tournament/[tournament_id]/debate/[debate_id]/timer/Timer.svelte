<script>
    export let targetTime; // in miliseconds
    export let timePassed; // in miliseconds

    /**
     * Formats seconds into a string of the form "mm:ss".
     * 
     * @param {number} s The number of seconds to format.
     * @returns {string} The formatted string.
     */
    function formatSeconds(s) {
        if (s < 0) {
            return `-${formatSeconds(-s)}`;
        }

        let minutes = Math.floor(s / 60);
        let seconds = s % 60;

        minutes = Math.floor(minutes);
        seconds = Math.floor(seconds);
        return `${minutes}:${seconds < 10 ? '0' : ''}${seconds}`;
    }
</script>

<style>
    .timer-container {
        display: flex;
        flex-direction: row;
        justify-content: center;
        align-items: center;

        font-size: 5rem;
    }
</style>

<div class="timer-container">
    {formatSeconds(targetTime - timePassed / 1000)}
</div>