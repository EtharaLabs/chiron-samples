/*
 * MIT License
 *
 * Copyright (c) 2021-2022 zodac.me
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

package me.zodac.folding.api.tc.stats;

import lombok.AccessLevel;
import lombok.AllArgsConstructor;
import lombok.EqualsAndHashCode;
import lombok.Getter;
import lombok.NoArgsConstructor;
import lombok.Setter;
import lombok.ToString;


@NoArgsConstructor
@AllArgsConstructor(access = AccessLevel.PRIVATE)
@Getter
@Setter
@EqualsAndHashCode
@ToString(doNotUseGetters = true)
public class OffsetTcStats {

    private static final long DEFAULT_POINTS = 0L;
    private static final long DEFAULT_MULTIPLIED_POINTS = 0L;
    private static final int DEFAULT_UNITS = 0;

    private long pointsOffset;
    private long multipliedPointsOffset;
    private int unitsOffset;

    
    public static OffsetTcStats create(final long pointsOffset, final long multipliedPointsOffset, final int unitsOffset) {
        return new OffsetTcStats(pointsOffset, multipliedPointsOffset, unitsOffset);
    }

    
    public static OffsetTcStats create(final UserTcStats userTcStats) {
        return create(userTcStats.getPoints(), userTcStats.getMultipliedPoints(), userTcStats.getUnits());
    }

    
    public static OffsetTcStats empty() {
        return create(DEFAULT_POINTS, DEFAULT_MULTIPLIED_POINTS, DEFAULT_UNITS);
    }

    
    public static OffsetTcStats updateWithHardwareMultiplier(final OffsetTcStats offsetTcStats, final double multiplier) {
        if (offsetTcStats.isEmpty() || !offsetTcStats.isMissingPointsOrMultipliedPoints()) {
            return offsetTcStats;
        }

        if (offsetTcStats.pointsOffset == DEFAULT_POINTS) {
            final long pointsOffset = Math.round(offsetTcStats.multipliedPointsOffset / multiplier);
            return create(pointsOffset, offsetTcStats.multipliedPointsOffset, offsetTcStats.unitsOffset);
        } else {
            final long multipliedPointsOffset = Math.round(offsetTcStats.pointsOffset * multiplier);
            return create(offsetTcStats.pointsOffset, multipliedPointsOffset, offsetTcStats.unitsOffset);
        }
    }

    
    public boolean isEmpty() {
        return pointsOffset == DEFAULT_POINTS && multipliedPointsOffset == DEFAULT_MULTIPLIED_POINTS && unitsOffset == DEFAULT_UNITS;
    }

    private boolean isMissingPointsOrMultipliedPoints() {
        return (pointsOffset == DEFAULT_POINTS) != (multipliedPointsOffset == DEFAULT_MULTIPLIED_POINTS);
    }
}
