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

package me.zodac.folding.api.db;

import java.time.Month;
import java.time.Year;
import java.util.Collection;
import java.util.Optional;
import me.zodac.folding.api.UserAuthenticationResult;
import me.zodac.folding.api.exception.DatabaseConnectionException;
import me.zodac.folding.api.tc.Hardware;
import me.zodac.folding.api.tc.Team;
import me.zodac.folding.api.tc.User;
import me.zodac.folding.api.tc.result.MonthlyResult;
import me.zodac.folding.api.tc.stats.OffsetTcStats;
import me.zodac.folding.api.tc.stats.RetiredUserTcStats;
import me.zodac.folding.api.tc.stats.UserStats;
import me.zodac.folding.api.tc.stats.UserTcStats;
import me.zodac.folding.rest.api.tc.historic.HistoricStats;


public interface DbManager {

    
    Hardware createHardware(final Hardware hardware);

    
    Collection<Hardware> getAllHardware();

    
    Optional<Hardware> getHardware(final int hardwareId);

    
    Hardware updateHardware(final Hardware hardwareToUpdate);

    
    void deleteHardware(final int hardwareId);

    
    Team createTeam(final Team team);

    
    Collection<Team> getAllTeams();

    
    Optional<Team> getTeam(final int teamId);

    
    Team updateTeam(final Team teamToUpdate);

    
    void deleteTeam(final int teamId);

    
    User createUser(final User user);

    
    Collection<User> getAllUsers();

    
    Optional<User> getUser(final int userId);

    
    User updateUser(final User userToUpdate);

    
    void deleteUser(final int userId);

    
    UserTcStats createHourlyTcStats(final UserTcStats userTcStats);

    
    Optional<UserTcStats> getHourlyTcStats(final int userId);

    
    Optional<UserTcStats> getFirstHourlyTcStats();

    
    Collection<HistoricStats> getHistoricStatsHourly(final int userId, final Year year, final Month month, final int day);

    
    Collection<HistoricStats> getHistoricStatsDaily(final int userId, final Year year, final Month month);

    
    Collection<HistoricStats> getHistoricStatsMonthly(final int userId, final Year year);

    
    UserStats createInitialStats(final UserStats userStats);

    
    Optional<UserStats> getInitialStats(final int userId);

    
    UserStats createTotalStats(final UserStats userStats);

    
    Optional<UserStats> getTotalStats(final int userId);

    
    OffsetTcStats createOrUpdateOffsetStats(final int userId, final OffsetTcStats offsetTcStats);

    
    Optional<OffsetTcStats> getOffsetStats(final int userId);

    
    void deleteOffsetStats(final int userId);

    
    void deleteAllOffsetStats();

    
    RetiredUserTcStats createRetiredUserStats(final RetiredUserTcStats retiredUserTcStats);

    
    Collection<RetiredUserTcStats> getAllRetiredUserStats();

    
    void deleteAllRetiredUserStats();

    
    MonthlyResult createMonthlyResult(final MonthlyResult monthlyResult);

    
    Optional<MonthlyResult> getMonthlyResult(final Month month, final Year year);

    
    UserAuthenticationResult authenticateSystemUser(final String userName, final String password);
}
