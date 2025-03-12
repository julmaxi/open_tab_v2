
<script>
    import { onMount, setContext } from "svelte";
    import { writable } from "svelte/store";

    let numSlides = 0;

    setContext("slide", {
        registerSlide: () => {
            numSlides += 1;
        },
        unregisterSlide: () => {
            numSlides -= 1;
        },
    });

	let currentSlide = writable(0);

    setContext("displaySlide", {
        currentSlide,
    });

    onMount(() => {
      window.addEventListener('keydown', handleKeyPress);
      return () => {
        window.removeEventListener('keydown', handleKeyPress);
      };
    });
  
    function handleKeyPress(event) {
      if (event.key === 'ArrowRight' || event.key === ' ' || event.key === 'Spacebar') {
        nextSlide();
      } else if (event.key === 'ArrowLeft') {
        prevSlide();
      }
    }
  
    function nextSlide() {
      if ($currentSlide < numSlides - 1) {
        $currentSlide++;
      };
    }
  
    function prevSlide() {
      if ($currentSlide > 0) {
        $currentSlide--;
      }
    }
</script>


<slot></slot>