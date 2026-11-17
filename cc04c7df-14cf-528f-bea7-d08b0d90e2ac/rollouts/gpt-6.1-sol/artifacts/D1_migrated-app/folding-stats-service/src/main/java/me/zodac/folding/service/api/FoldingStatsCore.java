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

package me.zodac.folding.service.api;

import java.time.Month;
import java.time.Year;
import java.util.Collection;
import java.util.Optional;
import me.zodac.folding.api.UserAuthenticationResult;
import me.zodac.folding.api.tc.Hardware;
import me.zodac.folding.api.tc.Team;
import me.zodac.folding.api.tc.User;
import me.zodac.folding.api.tc.result.MonthlyResult;
import me.zodac.folding.api.tc.stats.OffsetTcStats;
import me.zodac.folding.api.tc.stats.RetiredUserTcStats;
import me.zodac.folding.api.tc.stats.UserStats;
import me.zodac.folding.api.tc.stats.UserTcStats;
import me.zodac.folding.rest.api.tc.CompetitionSummary;
import me.zodac.folding.rest.api.tc.historic.HistoricStats;

public interface FoldingStatsCore {

    Hardware createHardware(final Hardware hardware);

    Optional<Hardware> getHardware(final int hardwareId);

    Collection<Hardware> getAllHardware();

    Hardware updateHardware(final Hardware hardwareToUpdate, final Hardware existingHardware);

    void deleteHardware(final Hardware hardware);

    Team createTeam(final Team team);

    Optional<Team> getTeam(final int teamId);

    Collection<Team> getAllTeams();

    Team updateTeam(final Team teamToUpdate);

    void deleteTeam(final Team team);

    User createUser(final User user);

    Optional<User> getUserWithPasskey(final int userId);

    Optional<User> getUserWithoutPasskey(final int userId);

    Collection<User> getAllUsersWithPasskeys();

    Collection<User> getAllUsersWithoutPasskeys();

    User updateUser(final User userToUpdate, final User existingUser);

    void deleteUser(final User user);

    Collection<User> getUsersOnTeam(final Team team);

    Collection<User> getUsersOnTeamWithPasskeys(final Team team);

    MonthlyResult createMonthlyResult(final MonthlyResult monthlyResult);

    Optional<MonthlyResult> getMonthlyResult(final Month month, final Year year);

    UserAuthenticationResult authenticateSystemUser(final String userName, final String password);

    Collection<HistoricStats> getHistoricStats(final User user, final Year year, final Month month, final int day);

    Collection<HistoricStats> getHistoricStats(final User user, final Year year, final Month month);

    Collection<HistoricStats> getHistoricStats(final User user, final Year year);

    UserStats createTotalStats(final UserStats userStats);

    UserStats getTotalStats(final User user);

    OffsetTcStats createOffsetStats(final User user, final OffsetTcStats offsetTcStats);

    OffsetTcStats createOrUpdateOffsetStats(final User user, final OffsetTcStats offsetTcStats);

    OffsetTcStats getOffsetStats(final User user);

    UserTcStats createHourlyTcStats(final UserTcStats userTcStats);

    UserTcStats getHourlyTcStats(final User user);

    boolean isAnyHourlyTcStatsExist();

    UserStats createInitialStats(final UserStats userStats);

    UserStats getInitialStats(final User user);

    RetiredUserTcStats createRetiredUserStats(final RetiredUserTcStats retiredUserTcStats);

    void resetAllTeamCompetitionUserStats();

    CompetitionSummary getCompetitionSummary();

    void printCacheContents();
}
