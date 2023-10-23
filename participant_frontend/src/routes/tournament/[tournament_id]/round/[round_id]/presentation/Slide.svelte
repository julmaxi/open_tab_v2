<script>
	import {getContext, onDestroy, onMount} from 'svelte';

    export let slideIndex;
    export let onShow = () => {};

	let { registerSlide, unregisterSlide } = getContext('slide');
    let { currentSlide } = getContext('displaySlide');
    onMount(() => {
		registerSlide();
	})

    currentSlide.subscribe((value) => {
        if (value === slideIndex) {
            onShow();
        }
    })

    onDestroy(() => {
		unregisterSlide();
    })
</script>


{#if $currentSlide == slideIndex}
    <slot></slot>
{/if}
