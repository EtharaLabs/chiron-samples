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

package me.zodac.folding.db.postgres;

import static me.zodac.folding.db.postgres.gen.tables.Hardware.HARDWARE;
import static me.zodac.folding.db.postgres.gen.tables.SystemUsers.SYSTEM_USERS;
import static me.zodac.folding.db.postgres.gen.tables.Teams.TEAMS;
import static me.zodac.folding.db.postgres.gen.tables.Users.USERS;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import java.time.LocalDateTime;
import java.util.Set;
import me.zodac.folding.api.UserAuthenticationResult;
import me.zodac.folding.api.tc.Category;
import me.zodac.folding.api.tc.Hardware;
import me.zodac.folding.api.tc.HardwareMake;
import me.zodac.folding.api.tc.HardwareType;
import me.zodac.folding.api.tc.Team;
import me.zodac.folding.api.tc.User;
import me.zodac.folding.api.tc.result.MonthlyResult;
import me.zodac.folding.api.tc.stats.OffsetTcStats;
import me.zodac.folding.api.tc.stats.RetiredUserTcStats;
import me.zodac.folding.api.tc.stats.UserStats;
import me.zodac.folding.api.tc.stats.UserTcStats;
import me.zodac.folding.api.util.DateTimeUtils;
import me.zodac.folding.db.postgres.gen.tables.records.HardwareRecord;
import me.zodac.folding.db.postgres.gen.tables.records.MonthlyResultsRecord;
import me.zodac.folding.db.postgres.gen.tables.records.RetiredUserStatsRecord;
import me.zodac.folding.db.postgres.gen.tables.records.TeamsRecord;
import me.zodac.folding.db.postgres.gen.tables.records.UserInitialStatsRecord;
import me.zodac.folding.db.postgres.gen.tables.records.UserOffsetTcStatsRecord;
import me.zodac.folding.db.postgres.gen.tables.records.UserTcStatsHourlyRecord;
import me.zodac.folding.db.postgres.gen.tables.records.UserTotalStatsRecord;
import me.zodac.folding.db.postgres.gen.tables.records.UsersRecord;
import me.zodac.folding.rest.api.tc.historic.HistoricStats;
import me.zodac.folding.rest.util.LocalDateTimeGsonTypeAdapter;
import org.jooq.Record;


final class RecordConverter {

    // We don't try and reuse the GSON instance available in RestUtilConstants
    // This is because we do not want pretty-print enabled since that would increase the size of the JSON string being persisted in the DB
    static final Gson MONTHLY_RESULT_GSON = new GsonBuilder()
        .registerTypeAdapter(LocalDateTime.class, LocalDateTimeGsonTypeAdapter.getInstance())
        .disableHtmlEscaping()
        .create();

    private RecordConverter() {

    }

    
    static Hardware toHardware(final HardwareRecord hardwareRecord) {
        return Hardware.create(
            hardwareRecord.getHardwareId(),
            hardwareRecord.getHardwareName(),
            hardwareRecord.getDisplayName(),
            HardwareMake.get(hardwareRecord.getHardwareMake()),
            HardwareType.get(hardwareRecord.getHardwareType()),
            hardwareRecord.getMultiplier().doubleValue(),
            hardwareRecord.getAveragePpd().longValue()
        );
    }

    
    static Team toTeam(final TeamsRecord teamRecord) {
        return Team.create(
            teamRecord.getTeamId(),
            teamRecord.getTeamName(),
            teamRecord.getTeamDescription(),
            teamRecord.getForumLink()
        );
    }

    
    static User toUser(final Record joinedRecord) {
        final UsersRecord userRecord = joinedRecord.into(USERS);
        return User.create(
            userRecord.getUserId(),
            userRecord.getFoldingUsername(),
            userRecord.getDisplayUsername(),
            userRecord.getPasskey(),
            Category.get(userRecord.getCategory()),
            userRecord.getProfileLink(),
            userRecord.getLiveStatsLink(),
            toHardware(joinedRecord.into(HARDWARE)),
            toTeam(joinedRecord.into(TEAMS)),
            userRecord.getIsCaptain()
        );
    }

    
    static UserTcStats toUserTcStats(final UserTcStatsHourlyRecord userTcStatsHourlyRecord) {
        return UserTcStats.create(
            userTcStatsHourlyRecord.getUserId(),
            DateTimeUtils.toTimestamp(userTcStatsHourlyRecord.getUtcTimestamp()),
            userTcStatsHourlyRecord.getTcPoints(),
            userTcStatsHourlyRecord.getTcPointsMultiplied(),
            userTcStatsHourlyRecord.getTcUnits()
        );
    }

    
    static HistoricStats toHistoricStats(final UserTcStatsHourlyRecord userTcStatsHourlyRecord) {
        return HistoricStats.create(
            userTcStatsHourlyRecord.getUtcTimestamp(),
            userTcStatsHourlyRecord.getTcPoints(),
            userTcStatsHourlyRecord.getTcPointsMultiplied(),
            userTcStatsHourlyRecord.getTcUnits()
        );
    }

    
    static UserStats toUserStats(final UserTotalStatsRecord userTotalStatsRecord) {
        return UserStats.create(
            userTotalStatsRecord.getUserId(),
            DateTimeUtils.toTimestamp(userTotalStatsRecord.getUtcTimestamp()),
            userTotalStatsRecord.getTotalPoints(),
            userTotalStatsRecord.getTotalUnits()
        );
    }

    
    static UserStats toUserStats(final UserInitialStatsRecord userInitialStatsRecord) {
        return UserStats.create(
            userInitialStatsRecord.getUserId(),
            DateTimeUtils.toTimestamp(userInitialStatsRecord.getUtcTimestamp()),
            userInitialStatsRecord.getInitialPoints(),
            userInitialStatsRecord.getInitialUnits()
        );
    }

    
    static OffsetTcStats toOffsetStats(final UserOffsetTcStatsRecord userOffsetTcStatsRecord) {
        return OffsetTcStats.create(
            userOffsetTcStatsRecord.getOffsetPoints(),
            userOffsetTcStatsRecord.getOffsetMultipliedPoints(),
            userOffsetTcStatsRecord.getOffsetUnits()
        );
    }

    
    static RetiredUserTcStats toRetiredUserStats(final RetiredUserStatsRecord retiredUserStatsRecord) {
        return RetiredUserTcStats.create(
            retiredUserStatsRecord.getRetiredUserId(),
            retiredUserStatsRecord.getTeamId(),
            retiredUserStatsRecord.getDisplayUsername(),
            UserTcStats.create(
                retiredUserStatsRecord.getUserId(),
                DateTimeUtils.toTimestamp(retiredUserStatsRecord.getUtcTimestamp()),
                retiredUserStatsRecord.getFinalPoints(),
                retiredUserStatsRecord.getFinalMultipliedPoints(),
                retiredUserStatsRecord.getFinalUnits()
            )
        );
    }

    
    static MonthlyResult toMonthlyResult(final MonthlyResultsRecord monthlyResultsRecord) {
        final MonthlyResult monthlyResult = MONTHLY_RESULT_GSON.fromJson(monthlyResultsRecord.getJsonResult(), MonthlyResult.class);
        return MonthlyResult.updateWithEmptyCategories(monthlyResult);
    }

    
    static UserAuthenticationResult toSystemUserAuthentication(final Record systemUsersRecord) {
        final boolean isPasswordMatch = getPasswordMatchValue(systemUsersRecord);

        if (isPasswordMatch) {
            final Set<String> roles = Set.of(systemUsersRecord.into(SYSTEM_USERS).getRoles());
            return UserAuthenticationResult.success(roles);
        }

        return UserAuthenticationResult.invalidPassword();
    }

    private static boolean getPasswordMatchValue(final Record systemUsersRecord) {
        final String passwordMatchFieldName = "is_password_match";
        return systemUsersRecord.get(passwordMatchFieldName) != null
            && systemUsersRecord.get(passwordMatchFieldName, boolean.class);
    }
}
