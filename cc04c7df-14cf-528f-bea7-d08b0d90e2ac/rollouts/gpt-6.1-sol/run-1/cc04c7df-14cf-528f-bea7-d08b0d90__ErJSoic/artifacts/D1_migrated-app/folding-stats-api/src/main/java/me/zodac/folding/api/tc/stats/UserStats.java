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
import lombok.ToString;
import me.zodac.folding.api.tc.User;
import me.zodac.folding.api.util.DateTimeUtils;


@EqualsAndHashCode(callSuper = true)
@ToString(doNotUseGetters = true, callSuper = true)
public class UserStats extends Stats {

    private final int userId;
    private final Timestamp timestamp;

    
    protected UserStats(final int userId, final Timestamp timestamp, final long points, final int units) {
        super(points, units);
        this.userId = userId;
        this.timestamp = new Timestamp(timestamp.getTime());
    }

    
    public static UserStats create(final int userId, final Timestamp timestamp, final long points, final int units) {
        return new UserStats(userId, timestamp, points, units);
    }

    
    public static UserStats empty() {
        return create(User.EMPTY_USER_ID, DateTimeUtils.currentUtcTimestamp(), Stats.DEFAULT_POINTS, Stats.DEFAULT_UNITS);
    }

    
    public int getUserId() {
        return userId;
    }

    
    public Timestamp getTimestamp() {
        return new Timestamp(timestamp.getTime());
    }

    @Override
    public boolean isEmpty() {
        return userId == User.EMPTY_USER_ID && super.isEmpty();
    }

    
    public boolean isEmptyStats() {
        return super.isEmpty();
    }
}