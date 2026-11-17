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

import lombok.EqualsAndHashCode;
import lombok.Getter;
import lombok.ToString;
import me.zodac.folding.api.tc.Team;
import me.zodac.folding.api.tc.User;


@Getter
@EqualsAndHashCode(callSuper = true)
@ToString(doNotUseGetters = true, callSuper = true)
public class RetiredUserTcStats extends UserTcStats {

    private static final String DEFAULT_DISPLAY_NAME = "retiredUser";
    private static final int EMPTY_RETIRED_USER_ID = 0;
    private static final int EMPTY_TEAM_ID = 0;

    private final int retiredUserId;
    private final int teamId;
    private final String displayName;

    
    protected RetiredUserTcStats(final int retiredUserId, final int teamId, final String displayName, final UserTcStats userTcStats) {
        super(userTcStats.getUserId(), userTcStats.getTimestamp(), userTcStats.getPoints(), userTcStats.getMultipliedPoints(),
            userTcStats.getUnits());
        this.retiredUserId = retiredUserId;
        this.teamId = teamId;
        this.displayName = displayName;
    }

    
    public static RetiredUserTcStats create(final int retiredUserId, final int teamId, final String displayName, final UserTcStats retiredTcStats) {
        return new RetiredUserTcStats(retiredUserId, teamId, displayName, retiredTcStats);
    }

    
    public static RetiredUserTcStats createWithoutId(final int teamId, final String displayName, final UserTcStats retiredTcStats) {
        return create(EMPTY_RETIRED_USER_ID, teamId, displayName, retiredTcStats);
    }

    
    public static RetiredUserTcStats updateWithId(final int retiredUserId, final RetiredUserTcStats retiredUserTcStats) {
        return create(
            retiredUserId,
            retiredUserTcStats.teamId,
            retiredUserTcStats.displayName,
            retiredUserTcStats
        );
    }
}