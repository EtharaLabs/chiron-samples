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

import java.sql.Timestamp;
import lombok.EqualsAndHashCode;
import lombok.Getter;
import lombok.ToString;
import me.zodac.folding.api.tc.User;
import me.zodac.folding.api.util.DateTimeUtils;


@Getter
@EqualsAndHashCode(callSuper = true)
@ToString(doNotUseGetters = true, callSuper = true)
public class UserTcStats extends UserStats {

    private static final long DEFAULT_MULTIPLIED_POINTS = 0L;

    private final long multipliedPoints;

    
    protected UserTcStats(final int userId, final Timestamp timestamp, final long points, final long multipliedPoints, final int units) {
        super(userId, timestamp, points, units);
        this.multipliedPoints = multipliedPoints;
    }

    
    public static UserTcStats create(final int userId, final Timestamp timestamp, final long points, final long multipliedPoints, final int units) {
        return new UserTcStats(userId, timestamp, points, multipliedPoints, units);
    }

    
    public static UserTcStats empty(final int userId) {
        return create(userId, DateTimeUtils.currentUtcTimestamp(), Stats.DEFAULT_POINTS, DEFAULT_MULTIPLIED_POINTS, Stats.DEFAULT_UNITS);
    }

    
    public static UserTcStats empty() {
        return empty(User.EMPTY_USER_ID);
    }

    
    public UserTcStats updateWithOffsets(final OffsetTcStats offsetTcStats) {
        final long offsetPoints = Math.max(getPoints() + offsetTcStats.getPointsOffset(), Stats.DEFAULT_POINTS);
        final long offsetMultipliedPoints = Math.max(multipliedPoints + offsetTcStats.getMultipliedPointsOffset(), DEFAULT_MULTIPLIED_POINTS);
        final int offsetUnits = Math.max(getUnits() + offsetTcStats.getUnitsOffset(), Stats.DEFAULT_UNITS);

        return create(getUserId(), getTimestamp(), offsetPoints, offsetMultipliedPoints, offsetUnits);
    }

    @Override
    public boolean isEmpty() {
        return multipliedPoints == DEFAULT_MULTIPLIED_POINTS && super.isEmpty();
    }
}