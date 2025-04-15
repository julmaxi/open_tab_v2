/**
 * @typedef {{x: number, y: number}} Vec2
 * @typedef {{x: number, y: number, width: number, height: number}} Rect
 */


function ticks(
    spacing
) {
    return (start, end) => {
        let ticks = [];
        let tick = Math.ceil(start / spacing) * spacing;
        while (tick <= end) {
            ticks.push({
                pos: tick,
                label: Math.round(tick * 1000) / 1000
            });
            tick += spacing;
        }

        return ticks;
    }
}

function autoTicks(forceInteger = false) {
    return (start, end) => {
        let range = end - start;
        let scale = Math.pow(10, Math.floor(Math.log10(range)) - 1);
        if (forceInteger && scale < 1) {
            scale = 1;
        }

        let stepSizes = [10, 5, 1];
        stepSizes = stepSizes.map(size => size * scale);
        let stepSize = stepSizes.find(size => range / size > 2);
        if (stepSize === undefined) {
            stepSize = 1;
        }
        let ticks = [];
        let tick = Math.ceil(start / stepSize) * stepSize;
        while (tick <= end) {
            ticks.push({
                pos: tick,
                label: Math.round(tick * 1000) / 1000
            });
            tick += stepSize;
        }   
        return ticks;
    }
}

export function dateTicks() {
    return (start, end) => {
        let range = end - start;
        if (range < 1000) {
            return [];
        }

        let scale = "seconds";
        let scales = [
            [1000, "seconds"],
            [60 * 1000, "minutes"],
            [60 * 60 * 1000, "hours"],
            [24 * 60 * 60 * 1000, "days"],
            [30 * 24 * 60 * 60 * 1000, "months"],
            [365 * 24 * 60 * 60 * 1000, "years"]
        ]

        for (let i = scales.length - 1; i >= 0; i--) {
            if (range / scales[i][0] > 2) {

                scale = scales[i][1];
                break;
            }
        }
        let startDate = new Date(start);
        let currDate = new Date(start);
        let endDate = new Date(end);

        let ticks = [];
        let step = 1;
        switch (scale) {
            case "seconds":
                let numSeconds = Math.floor((end - start) / 1000);
                currDate.setMilliseconds(0);
                step = 1;
                if (numSeconds >= 60) {
                    currDate.setSeconds(0);
                    step = 30;
                }
                else if (numSeconds >= 30) {
                    currDate.setSeconds(0);
                    step = 15;
                }
                else if (numSeconds >= 8) {
                    currDate.setSeconds(0);
                    step = 5;
                }
                
                while (currDate <= endDate) {
                    ticks.push({
                        pos: currDate.getTime(),
                        label: currDate.toLocaleTimeString()
                    });
                    currDate.setSeconds(currDate.getSeconds() + step);
                }
                break;
            case "minutes":
                let numMinutes = Math.floor((end - start) / (1000 * 60));
                currDate.setMilliseconds(0);
                currDate.setSeconds(0);

                step = 1;
                if (numMinutes >= 60) {
                    currDate.setMinutes(0);
                    step = 30;
                }
                else if (numMinutes >= 30) {
                    currDate.setMinutes(0);
                    step = 15;
                }
                else if (numMinutes >= 10) {
                    currDate.setMinutes(0);
                    step = 5;
                }

                while (currDate <= endDate) {
                    ticks.push({
                        pos: currDate.getTime(),
                        label: currDate.toLocaleTimeString(
                            undefined,
                            {
                                hour: '2-digit',
                                minute: '2-digit'
                            }
                        )
                    });
                    currDate.setMinutes(currDate.getMinutes() + step);
                }
                break;
            case "hours":
                currDate.setMilliseconds(0);
                currDate.setSeconds(0);
                currDate.setMinutes(0);

                let numHours = Math.floor((end - start) / (1000 * 60 * 60));
                if (numHours >= 24) {
                    currDate.setHours(0);
                    step = 6;
                }
                else if (numHours >= 12) {
                    currDate.setHours(0);
                    step = 3;
                }
                else if (numHours >= 6) {
                    currDate.setHours(0);
                    step = 2;
                }
                
                while (currDate <= endDate) {
                    ticks.push({
                        pos: currDate.getTime(),
                        label: currDate.toLocaleTimeString(
                            undefined,
                            {
                                hour: '2-digit',
                                minute: '2-digit'
                            }
                        )
                    });
                    currDate.setHours(currDate.getHours() + step);
                }
                break;
            case "days":
                currDate.setMilliseconds(0);
                currDate.setSeconds(0);
                currDate.setMinutes(0);
                currDate.setHours(0);
                let numDays = Math.floor((end - start) / (1000 * 60 * 60 * 24));

                if (numDays >= 31) {
                    currDate.setDate(1);
                    step = 14;
                }
                else if (numDays >= 6) {
                    currDate.setDate(1);
                    step = 7;
                }

                let prevMonth = currDate.getMonth();

                while (currDate <= endDate) {
                    if (step > 1 && currDate.getDate() >= 28) {
                        currDate.setMonth(currDate.getMonth() + 1);
                        currDate.setDate(1);
                    }
                    ticks.push({
                        pos: currDate.getTime(),
                        label: currDate.toLocaleDateString(undefined, {
                            year: 'numeric',
                            month: 'short',
                            day: '2-digit'
                        })
                    });
                    currDate.setDate(currDate.getDate() + step);
                    if (currDate.getMonth() !== prevMonth) {
                        prevMonth = currDate.getMonth();
                        currDate.setDate(1);
                    }
                }
                break;
            case "months":
                currDate.setMilliseconds(0);
                currDate.setSeconds(0);
                currDate.setMinutes(0);
                currDate.setHours(0);
                currDate.setDate(1);

                let numMonths = (endDate.getFullYear() - startDate.getFullYear()) * 12 - startDate.getMonth() + endDate.getMonth();

                if (numMonths >= 12) {
                    currDate.setMonth(0);
                    step = 6;
                }
                else if (numMonths >= 6) {
                    currDate.setMonth(0);
                    step = 3;
                }
                while (currDate <= endDate) {
                    ticks.push({
                        pos: currDate.getTime(),
                        //Show month and year
                        label: currDate.toLocaleDateString(undefined, {
                            year: 'numeric',
                            month: 'short'
                        })
                    });
                    currDate.setMonth(currDate.getMonth() + step);
                }
                break;
            case "years":
                currDate.setMilliseconds(0);
                currDate.setSeconds(0);
                currDate.setMinutes(0);
                currDate.setHours(0);
                currDate.setDate(1);
                currDate.setMonth(0);

                ticks = autoTicks(true)(startDate.getFullYear(), endDate.getFullYear());
                ticks = ticks.map(tick => {
                    return {
                        pos: new Date(tick.pos, 0, 1).getTime(),
                        label: tick.label
                    }
                });
                break;
            default:
                return [];
        }

        while (ticks.length > 0 && ticks[0].pos < start) {
            ticks.shift();
        }

        return ticks;
    }
}

/**
 * @property {Rect} viewportSize
 * @property {Rect} windowRect
 */
export class Plot {
    /** 
     * @param {Rect} viewportSize
     * @param {Rect} windowRect
     */
    constructor(
        viewportSize,
        windowRect,
        options = {}
    ) {
        this._viewportRect = viewportSize;
        this._windowRect = windowRect;

        this._insetLeft = 25.0;
        this._insetBottom = 20.0;
        this._insetTop = 5.0;
        this._insetRight = 10.0;

        this._xPixelForUnit = 0.0;
        this._yPixelForUnit = 0.0;

        this._recomputeDimensions();

        this.xTicks = options.xTicks || autoTicks();
        this.yTicks = options.yTicks || autoTicks();

        this.children = [];
    }

    _recomputeDimensions() {
        this._dataRect = {
            x: this._insetLeft + this._viewportRect.x,
            y: this._viewportRect.y + this._insetTop,
            width: this._viewportRect.width - this._insetLeft - this._insetRight,
            height: this._viewportRect.height - this._insetBottom - this._insetTop
        }

        this._xPixelForUnit = this._dataRect.width / this._windowRect.width;
        this._yPixelForUnit = this._dataRect.height / this._windowRect.height;
    }

    /**
     * 
     * @param {Vec2} unit 
     * @returns 
     */
    unitToPixel(unit) {
        return {
            x: (unit.x - this.windowRect.x) * this._xPixelForUnit + this._viewportRect.x + this._insetLeft,
            y: this._viewportRect.height - ((unit.y - this.windowRect.y) * this._yPixelForUnit) + this._viewportRect.y - this._insetBottom
        }
    }

    pixelToUnit(pixel) {
        return {
            x: (pixel.x - this._viewportRect.x - this._insetLeft) / this._xPixelForUnit + this.windowRect.x,
            y: this.windowRect.y + (this._viewportRect.height - pixel.y + this._insetBottom) / this._yPixelForUnit
        }
    }

    /**
     * 
     * @param {CanvasRenderingContext2D} ctx 
     */
    render(ctx) {
        ctx.save();
        ctx.reset();
        const dpi = window.devicePixelRatio;
        ctx.scale(dpi, dpi);

        ctx.clearRect(
            this._viewportRect.x,
            this._viewportRect.y,
            this._viewportRect.width,
            this._viewportRect.height
        );

        ctx.beginPath();
        ctx.rect(
            this._dataRect.x,
            this._dataRect.y,
            this._dataRect.width,
            this._dataRect.height
        );
        ctx.closePath();
        ctx.fillStyle = 'rgb(255, 255, 255)';
        ctx.fill();

        ctx.fillStyle = 'rgb(120, 0, 0)';

        //this.renderGrid(ctx);

        ctx.rect(
            this._viewportRect.x,
            this._viewportRect.y,
            this._viewportRect.width,
            this._viewportRect.height
        );
        ctx.closePath();
        ctx.clip();

        this.renderGrid(ctx);

        ctx.rect(
            this._dataRect.x,
            this._dataRect.y,
            this._dataRect.width,
            this._dataRect.height
        )
        ctx.closePath();
        ctx.clip();

        for (let child of this.children) {
            child.render(ctx, this);
        }
    }

    set windowRect(rect) {
        this._windowRect = rect;
        this._recomputeDimensions();
    }
    
    get windowRect() {
        return this._windowRect;
    }

    get xPixelForUnit() {
        return this._xPixelForUnit;
    }

    get yPixelForUnit() {
        return this._yPixelForUnit;
    }

    addChild(child) {
        this.children.push(child);
    }

    /**
     * 
     * @param {CanvasRenderingContext2D} ctx 
     */
    renderGrid(ctx) {
        ctx.font = "10px Lato, sans-serif";
        let lineWidth = 1.0;
        ctx.lineWidth = lineWidth;
        let topLeft = this.unitToPixel({
            x: this._windowRect.x,
            y: this._windowRect.height + this._windowRect.y
        });
        let bottomRight = this.unitToPixel({
            x: this._windowRect.width + this._windowRect.x,
            y: this._windowRect.y
        });

        ctx.save();
        ctx.fillStyle = 'rgb(0, 0, 0)';
        ctx.strokeStyle = '#ddd';

        ctx.beginPath();
        ctx.rect(
            topLeft.x - lineWidth/2,
            topLeft.y - lineWidth/2,
            bottomRight.x - topLeft.x + lineWidth,
            bottomRight.y - topLeft.y + lineWidth
        );
        ctx.stroke();

        let xTicks = this.xTicks(
            this._windowRect.x,
            this._windowRect.width + this._windowRect.x
        );

        ctx.beginPath();
        ctx.lineWidth = 1.0;
        for (let tick of xTicks) {
            let pos = this.unitToPixel({
                x: tick.pos,
                y: this._windowRect.y
            });
            ctx.moveTo(
                pos.x,
                pos.y
            )
            ctx.lineTo(
                pos.x,
                this._dataRect.y
            );

            let labelStats = ctx.measureText(tick.label);

            ctx.fillText(
                tick.label,
                pos.x - labelStats.width / 2,
                this._dataRect?.y + this._dataRect?.height + labelStats.fontBoundingBoxAscent + 3.0
            )
        }
        ctx.stroke();

        let yTicks = autoTicks()(
            this._windowRect.y,
            this._windowRect.height + this._windowRect.y
        );
        ctx.beginPath();
        ctx.lineWidth = 1.0;
        for (let tick of yTicks) {
            let pos = this.unitToPixel({
                x: this._windowRect.x,
                y: tick.pos
            });
            
            ctx.moveTo(
                pos.x,
                pos.y
            )
            ctx.lineTo(
                this._dataRect.x + this._dataRect.width,
                pos.y
            );

            let labelStats = ctx.measureText(tick.label);

            ctx.fillText(
                tick.label,
                this._dataRect?.x - labelStats.width - 3.0,
                pos.y + labelStats.actualBoundingBoxAscent / 2
            )
        }
        ctx.stroke();

        ctx.restore();
    }
}

export class InteractivePlot {
    constructor(
        canvas,
        windowRect,
        constraintRect,
        options = {}
    ) {

        this._constraintRect = constraintRect;
        let viewportSize = {
            x: 0,
            y: 0,
            width: canvas.width / window.devicePixelRatio,
            height: canvas.height / window.devicePixelRatio
        }
        this.constrainRect(windowRect);
        this._plot = new Plot(
            viewportSize,
            windowRect,
            options
        );
        this._plot.render(canvas.getContext('2d'));

        this._canvas = canvas;

        this._isMouseDown = false;
        this.lastMousePos = {
            x: 0,
            y: 0
        }

        this._rerenderCallback = null;

        this._pointerDownHandler = (event) => {
            this._isMouseDown = true;
            this.lastMousePos.x = event.clientX;
            this.lastMousePos.y = event.clientY;
        };

        this._pointerUpHandler = (event) => {
            this._isMouseDown = false;
        };

        this._pointerMoveHandler = (event) => {
            if (this._isMouseDown) {
                let deltaX = event.clientX - this.lastMousePos.x;
                let deltaY = event.clientY - this.lastMousePos.y;

                this.lastMousePos.x = event.clientX;
                this.lastMousePos.y = event.clientY;

                let adjustedDeltaX = deltaX / this._plot.xPixelForUnit;
                let adjustedDeltaY = deltaY / this._plot.yPixelForUnit;

                let newWindowRect = {
                    x: options.lockXPan ? this._plot.windowRect.x : this._plot.windowRect.x - adjustedDeltaX,
                    y: options.lockYPan ? this._plot.windowRect.y : this._plot.windowRect.y + adjustedDeltaY,
                    width: this._plot.windowRect.width,
                    height: this._plot.windowRect.height
                };

                if (newWindowRect.x < this._constraintRect.x) {
                    newWindowRect.x = this._constraintRect.x;
                }
                if (newWindowRect.y < this._constraintRect.y) {
                    newWindowRect.y = this._constraintRect.y;
                }
                if (newWindowRect.x + newWindowRect.width > this._constraintRect.x + this._constraintRect.width) {
                    newWindowRect.x = this._constraintRect.x + this._constraintRect.width - newWindowRect.width;
                }
                if (newWindowRect.y + newWindowRect.height > this._constraintRect.y + this._constraintRect.height) {
                    newWindowRect.y = this._constraintRect.y + this._constraintRect.height - newWindowRect.height;
                }

                this._plot.windowRect = newWindowRect;

                if (!this._rerenderCallback) {
                    this._rerenderCallback = requestAnimationFrame(() => {
                        this._plot.render(this._canvas.getContext('2d'));
                        this._rerenderCallback = null;
                    });
                }
            }
        };

        this._wheelHandler = (event) => {
            event.preventDefault();
            let xScale = this._plot.xPixelForUnit;
            let yScale = this._plot.yPixelForUnit;
            let w = this._plot.windowRect;
            let deltaX = event.deltaY / xScale;
            let deltaY = event.deltaY / yScale;
            if (!options.lockXZoom) {
                w.x += deltaX * 0.05;
                w.width -= deltaX * 0.05;
            }
            if (!options.lockYZoom) {
                w.y += deltaY * 0.05;
                w.height -= deltaY * 0.05;
            }

            this.constrainRect(w);
            this._plot.windowRect = w;

            if (!this._rerenderCallback) {
                this._rerenderCallback = requestAnimationFrame(() => {
                    this._plot.render(this._canvas.getContext('2d'));
                    this._rerenderCallback = null;
                });
            }
        };

        let initialDistance = null;

        function getDistance(touches) {
          const [touch1, touch2] = touches;
          const dx = touch1.clientX - touch2.clientX;
          const dy = touch1.clientY - touch2.clientY;
          return Math.sqrt(dx * dx + dy * dy);
        }
        
        canvas.addEventListener('touchstart', (e) => {
          if (e.touches.length === 2) {
            initialDistance = getDistance(e.touches);
          }
        }, { passive: false });
        
        canvas.addEventListener('touchmove', (e) => {
          if (e.touches.length === 2 && initialDistance !== null) {
            const currentDistance = getDistance(e.touches);
            if (Math.abs(currentDistance - initialDistance) > 10) {
              console.log('Pinch zoom detected');
            }
            e.preventDefault(); // Optional: prevent native zoom
          }
        }, { passive: false });
        
        canvas.addEventListener('touchend', () => {
          initialDistance = null;
        });
        
        this.onClickPoint = null;

        this._clickHandler = (event) => {
            if (!this.onClickPoint) {
                return;
            }
            let selectedPoint = {
                x: event.clientX - this._canvas.getBoundingClientRect().x,
                y: event.clientY - this._canvas.getBoundingClientRect().y
            };
            
            for (let child of this._plot.children) {
                let p = child.getClosestPointPixels({
                    x: selectedPoint.x,
                    y: selectedPoint.y
                }, this._plot);

                if (p) {
                    event.stopPropagation();
                    return this.onClickPoint({
                        index: p.index,
                        position: p.position
                    });
                }
            }
        }

        this._canvas.addEventListener('pointerdown', this._pointerDownHandler);
        window.addEventListener('pointerup', this._pointerUpHandler);
        window.addEventListener('pointermove', this._pointerMoveHandler);
        this._canvas.addEventListener('wheel', this._wheelHandler);
        this._canvas.addEventListener('click', this._clickHandler);
    }

    constrainRect(
        newWindowRect
    ) {
        newWindowRect.x = Math.max(newWindowRect.x, this._constraintRect.x);
        newWindowRect.y = Math.max(newWindowRect.y, this._constraintRect.y);

        newWindowRect.width = Math.min(newWindowRect.width, this._constraintRect.x + this._constraintRect.width - newWindowRect.x);
        newWindowRect.height = Math.min(newWindowRect.height, this._constraintRect.y + this._constraintRect.height - newWindowRect.y);
    }
    
    addChild(child) {
        this._plot.addChild(child);
        if (!this._rerenderCallback) {
            this._rerenderCallback = requestAnimationFrame(() => {
                this._plot.render(this._canvas.getContext('2d'));
                this._rerenderCallback = null;
            });
        }
    }

    close() {
        if (this._canvas) {
            this._canvas.removeEventListener('pointerdown', this._pointerDownHandler);
            window.removeEventListener('pointerup', this._pointerUpHandler);
            window.removeEventListener('pointermove', this._pointerMoveHandler);
            this._canvas.removeEventListener('wheel', this._wheelHandler);
            this._canvas.removeEventListener('click', this._clickHandler);
            this._canvas = null;
        }
    }
}

export class PointsGraph {
    constructor(
        points
    ) {
        this._points = points;
    }

    render(ctx, plot) {
        ctx.save();

        ctx.fillStyle = 'rgb(0, 0, 0)';
        ctx.strokeStyle = 'rgb(0, 0, 0)';

        const pointPaths = [];
        const linePath = new Path2D();

        let startIdx = 0;

        let pixelPositions = this._points.map((point) => {
            return plot.unitToPixel(point);
        });

        while (
            startIdx < this._points.length
            &&
            pixelPositions[startIdx].x < plot._dataRect.x
        ) {
            startIdx++;
        }

        let endIdx = this._points.length - 1;

        while (
            endIdx >= startIdx
            &&
            endIdx > 0
            &&
            pixelPositions[endIdx].x > plot._dataRect.x + plot._dataRect.width
        ) {
            endIdx--;
        }

        if (startIdx > 0) {
            let slope = (
                pixelPositions[startIdx - 1].y - pixelPositions[startIdx].y
            ) / (pixelPositions[startIdx - 1].x - pixelPositions[startIdx].x);
            let axisIntercept = pixelPositions[startIdx - 1].y + slope * (plot._dataRect.x - pixelPositions[startIdx - 1].x);
            linePath.moveTo(
                plot._dataRect.x,
                axisIntercept
            )
        }

        for (let i = startIdx; i <= endIdx; i++) {
            let point = this._points[i];
            let pos = plot.unitToPixel(point);
            const pointsPath = new Path2D();
            pointsPath.moveTo(pos.x + 4.0, pos.y);
            switch (point.symbol) {
                case 'square':
                    pointsPath.rect(pos.x - 4.0, pos.y - 4.0, 8.0, 8.0);
                    break;
                case 'triangle':
                    pointsPath.moveTo(pos.x - 4.0, pos.y + 4.0);
                    pointsPath.lineTo(pos.x + 4.0, pos.y + 4.0);
                    pointsPath.lineTo(pos.x, pos.y - 4.0);
                    break;
                case 'pentagon':
                    {
                        const r = 5.0;
                        const angleOffset = -Math.PI / 2;
                        const vertices = [];
                        for (let i = 0; i < 5; i++) {
                            const angle = angleOffset + i * 2 * Math.PI / 5;
                            vertices.push({
                                x: pos.x + r * Math.cos(angle),
                                y: pos.y + r * Math.sin(angle)
                            });
                        }
                        pointsPath.moveTo(vertices[0].x, vertices[0].y);
                        for (let i = 1; i < vertices.length; i++) {
                            pointsPath.lineTo(vertices[i].x, vertices[i].y);
                        }
                        pointsPath.closePath();
                    }
                    break;
                default:
                    pointsPath.arc(pos.x, pos.y, 4.0, 0, Math.PI * 2);
                    break;
            }
            let color = point.color || '#131980';
            pointsPath.closePath();
            pointPaths.push({p: pointsPath, c: color});

            if (i == 0) {
                linePath.moveTo(pos.x, pos.y);
            }
            else {
                linePath.lineTo(pos.x, pos.y);
            }
        }

        if (endIdx < this._points.length - 1) {
            let slope = (
                pixelPositions[endIdx].y - pixelPositions[endIdx + 1].y
            ) / (pixelPositions[endIdx].x - pixelPositions[endIdx + 1].x);
            let axisIntercept = pixelPositions[endIdx].y + slope * (plot._dataRect.x + plot._dataRect.width - pixelPositions[endIdx].x);
            linePath.lineTo(
                plot._dataRect.x + plot._dataRect.width,
                axisIntercept
            )
        }

        ctx.lineJoin = 'bevel';
        ctx.lineWidth = 2.0;
        ctx.strokeStyle = 'rgb(0, 0, 0)';
        ctx.stroke(linePath);

        for (let i = 0; i < pointPaths.length; i++) {
            const pointsPath = pointPaths[i].p;
            ctx.lineWidth = 1.0;
            ctx.fillStyle = pointPaths[i].c;
            ctx.beginPath();
            ctx.fill(pointsPath);
            ctx.stroke(pointsPath);
            ctx.closePath();
        }
        ctx.closePath();
        ctx.restore();
    }

    getClosestPointPixels(
        pos,
        plot,
        limit=5
    ) {
        const closestPoints = this._points.map((point, index) => {
            const pointPos = plot.unitToPixel(point);
            const distance = Math.sqrt(Math.pow(pos.x - pointPos.x, 2) + Math.pow(pos.y - pointPos.y, 2));
            return { index, distance, position: pointPos };
        });

        closestPoints.sort((a, b) => a.distance - b.distance);
        if (closestPoints[0].distance <= limit) {
            return closestPoints[0];
        }
        return null;
    }
}


function computeStd(points) {
    let mean = points.reduce((acc, point) => acc + point, 0) / points.length;
    let variance = points.reduce((acc, point) => acc + Math.pow(point - mean, 2), 0) / points.length;
    return Math.sqrt(variance);
}


function gaussianRandom(mean=0, stdev=1) {
    const u = 1 - Math.random(); // Converting [0,1) to (0,1]
    const v = Math.random();
    const z = Math.sqrt( -2.0 * Math.log( u ) ) * Math.cos( 2.0 * Math.PI * v );
    // Transform to the desired mean and standard deviation:
    return z * stdev + mean;
}

export class KDEPlot {
    constructor(
        points,
        options = {}
    ) {
        this._points = points;

        this._std = computeStd(this._points);
        this.bandwidth = 1.06 * this._std * Math.pow(this._points.length, -1.0 / 5.0);
        this._mult = (
            1.0/
            (
                this._points.length * this.bandwidth * this._std * Math.sqrt(2.0 * Math.PI)
            )
        )
    }

    render(ctx, plot) {
        ctx.save();

        let path = new Path2D();

        let vals = [];
        for (
            let pixel = plot._dataRect.x;
            pixel <= plot._dataRect.x + plot._dataRect.width;
            pixel += 1.0
        ) {
            let xVal = plot.pixelToUnit({
                x: pixel,
                y: 0
            }).x;

            let kernelSum = 0;
            for (let point of this._points) {
                let kernel = Math.exp(
                    -Math.pow((xVal - point), 2)
                    /
                    (2.0 * this.bandwidth * this.bandwidth * this._std * this._std)
                );     
                kernelSum += kernel;           
            }

            vals.push([pixel, kernelSum * this._mult])
        }

        let m = vals.reduce((acc, val) => Math.max(acc, val[1]), 0);

        path.moveTo(0, plot._dataRect.y + plot._dataRect.height);
        for (let [pixel, val] of vals) {
            path.lineTo(pixel, plot.unitToPixel({
                x: 0,
                y: (val / (m))
            }).y);
        }
        ctx.fillStyle = 'rgb(0, 0, 0)';
        ctx.strokeStyle = 'rgb(0, 0, 0)';
        ctx.lineWidth = 1.0;
        ctx.stroke(path);

        path.lineTo(
            plot._dataRect.x + plot._dataRect.width,
            plot._dataRect.y + plot._dataRect.height
        );
        path.closePath()

        //ctx.fillStyle = 'rgba(0 14 171, 0.5)';
        //Correct:
        ctx.fillStyle = 'rgba(0, 14, 171, 0.2)';

        ctx.fill(path);
    }
}