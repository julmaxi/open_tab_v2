<script>
	export let number;
    export let locale = "de";

	$: formattedNumber = number.toLocaleString(locale, {
		notation: "standard",
        minimumFractionDigits: 2,
        maximumFractionDigits: 2,
	});
</script>

<style>
	span {
		font-variant-numeric: tabular-nums;
	}
</style>

<span>{formattedNumber}</span>
