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

package me.zodac.folding.rest.api.tc.historic;

import java.time.LocalDateTime;
import java.time.temporal.ChronoUnit;
import java.util.ArrayList;
import java.util.Collection;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.TreeSet;
import java.util.stream.Collectors;
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
public class HistoricStats {

    private static final long DEFAULT_POINTS = 0L;
    private static final long DEFAULT_MULTIPLIED_POINTS = 0L;
    private static final int DEFAULT_UNITS = 0;

    private LocalDateTime dateTime;
    private long points;
    private long multipliedPoints;
    private int units;

    
    public static HistoricStats create(final LocalDateTime dateTime, final long points, final long multipliedPoints, final int units) {
        return new HistoricStats(dateTime.truncatedTo(ChronoUnit.HOURS), points, multipliedPoints, units);
    }

    
    public static Collection<HistoricStats> combine(final Collection<HistoricStats> allStats) {
        final Set<LocalDateTime> keys = allStats.stream()
            .map(HistoricStats::getDateTime)
            .sorted()
            .collect(Collectors.toCollection(TreeSet::new));

        final List<HistoricStats> combinedStats = new ArrayList<>();

        for (final LocalDateTime key : keys) {
            long combinedPoints = DEFAULT_POINTS;
            long combinedMultipliedPoints = DEFAULT_MULTIPLIED_POINTS;
            int combinedUnits = DEFAULT_UNITS;

            for (final HistoricStats stats : allStats) {
                if (stats.dateTime.equals(key)) {
                    combinedPoints += stats.points;
                    combinedMultipliedPoints += stats.multipliedPoints;
                    combinedUnits += stats.units;
                }
            }

            combinedStats.add(create(key, combinedPoints, combinedMultipliedPoints, combinedUnits));
        }

        return combinedStats;
    }
}
