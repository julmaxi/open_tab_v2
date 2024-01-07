<script>
    export let targetTime; // in seconds
    export let startTime; // in miliseconds
    export let currTime; // in miliseconds

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
    {#if startTime === null}
        {formatSeconds(targetTime)}
    {:else}
        {formatSeconds(targetTime - (currTime - startTime) / 1000)}
    {/if}
</div>