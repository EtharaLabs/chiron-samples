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

package me.zodac.folding.api.util;

import java.sql.Timestamp;
import java.time.Instant;
import java.time.LocalDateTime;
import java.time.Month;
import java.time.OffsetDateTime;
import java.time.Year;
import java.time.ZoneOffset;
import java.time.temporal.ChronoUnit;
import java.util.Date;
import java.util.Locale;


public final class DateTimeUtils {

    private DateTimeUtils() {

    }

    
    public static int untilNextMonthUtc(final ChronoUnit chronoUnit) {
        final int currentMonth = currentUtcMonth().getValue();
        final int nextMonth = currentMonth == 12 ? 1 : (currentMonth + 1);
        final int currentYear = currentUtcYear().getValue();
        final int nextMonthYear = currentMonth == 12 ? (currentYear + 1) : currentYear;

        final String nextMonthAsDate = String.format("%s-%02d-01T00:00:00Z", nextMonthYear, nextMonth);

        return (int) chronoUnit.between(
            Instant.now().atZone(ZoneOffset.UTC),
            Instant.parse(nextMonthAsDate).atZone(ZoneOffset.UTC)
        );
    }

    
    public static String formatMonth(final Month month) {
        final String monthAsString = month.toString();
        return monthAsString.substring(0, 1).toUpperCase(Locale.UK) + monthAsString.substring(1).toLowerCase(Locale.UK);
    }

    
    public static LocalDateTime getLocalDateTimeOf(final Year year, final Month month, final int day, final int hour, final int minute,
                                                   final int second) {
        return LocalDateTime.of(year.getValue(), month.getValue(), day, hour, minute, second);
    }

    
    public static LocalDateTime getLocalDateTimeOf(final Year year, final Month month) {
        return LocalDateTime.of(year.getValue(), month.getValue(), 1, 0, 0, 0);
    }

    
    public static Timestamp getTimestampOf(final Year year, final Month month, final int day, final int hour, final int minute, final int second) {
        return toTimestamp(LocalDateTime.of(year.getValue(), month.getValue(), day, hour, minute, second));
    }

    
    public static Timestamp toTimestamp(final LocalDateTime localDateTime) {
        return new Timestamp(localDateTime.toInstant(ZoneOffset.UTC).toEpochMilli());
    }

    
    public static LocalDateTime toUtcLocalDateTime(final Date date) {
        return date
            .toInstant()
            .atOffset(ZoneOffset.UTC)
            .toLocalDateTime();
    }

    
    public static Timestamp currentUtcTimestamp() {
        return new Timestamp(currentUtcDateTime().toInstant().toEpochMilli());
    }

    
    public static Month currentUtcMonth() {
        return currentUtcDateTime().toLocalDate().getMonth();
    }

    
    public static Year currentUtcYear() {
        return Year.now(ZoneOffset.UTC);
    }

    
    public static OffsetDateTime currentUtcDateTime() {
        return OffsetDateTime.now(ZoneOffset.UTC);
    }
}
