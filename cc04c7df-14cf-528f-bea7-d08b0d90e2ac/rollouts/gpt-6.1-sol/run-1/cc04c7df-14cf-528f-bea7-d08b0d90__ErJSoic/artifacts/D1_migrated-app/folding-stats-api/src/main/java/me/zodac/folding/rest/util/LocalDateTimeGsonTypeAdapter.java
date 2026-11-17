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

package me.zodac.folding.rest.util;

import com.google.gson.JsonDeserializationContext;
import com.google.gson.JsonDeserializer;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonSerializationContext;
import com.google.gson.JsonSerializer;
import java.lang.reflect.Type;
import java.time.LocalDateTime;


public final class LocalDateTimeGsonTypeAdapter implements JsonDeserializer<LocalDateTime>, JsonSerializer<LocalDateTime> {

    private static final LocalDateTimeGsonTypeAdapter INSTANCE = new LocalDateTimeGsonTypeAdapter();

    private LocalDateTimeGsonTypeAdapter() {

    }

    
    public static LocalDateTimeGsonTypeAdapter getInstance() {
        return INSTANCE;
    }

    @Override
    public LocalDateTime deserialize(final JsonElement json, final Type typeOfT, final JsonDeserializationContext context) {
        final JsonObject date = json.getAsJsonObject().get("date").getAsJsonObject();
        final JsonObject time = json.getAsJsonObject().get("time").getAsJsonObject();

        final int year = date.get("year").getAsInt();
        final int month = date.get("month").getAsInt();
        final int dayOfMonth = date.get("day").getAsInt();

        final int hour = time.get("hour").getAsInt();
        final int minute = time.get("minute").getAsInt();
        final int second = time.get("second").getAsInt();
        final int nano = time.get("nano").getAsInt();

        return LocalDateTime.of(year, month, dayOfMonth, hour, minute, second, nano);
    }

    @Override
    public JsonElement serialize(final LocalDateTime localDateTime, final Type typeOfSrc, final JsonSerializationContext context) {
        final JsonObject output = new JsonObject();

        final JsonObject date = new JsonObject();
        date.addProperty("year", localDateTime.getYear());
        date.addProperty("month", localDateTime.getMonthValue());
        date.addProperty("day", localDateTime.getDayOfMonth());

        final JsonObject time = new JsonObject();
        time.addProperty("hour", localDateTime.getHour());
        time.addProperty("minute", localDateTime.getMinute());
        time.addProperty("second", localDateTime.getSecond());
        time.addProperty("nano", localDateTime.getNano());

        output.add("date", date);
        output.add("time", time);
        return output;
    }
}
