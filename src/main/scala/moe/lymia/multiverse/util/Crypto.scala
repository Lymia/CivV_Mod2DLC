/*
 * Copyright (c) 2015-2016 Lymia Alusyia <lymia@lymiahugs.com>
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in
 * all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
 * THE SOFTWARE.
 */

package moe.lymia.multiverse.util

import java.nio.ByteBuffer
import java.security.MessageDigest
import java.util.UUID

object Crypto {
  def digest(algorithm: String, data: Seq[Byte]) = {
    val md = MessageDigest.getInstance(algorithm)
    val hash = md.digest(data.toArray)
    hash
  }
  def hexdigest(algorithm: String, data: Seq[Byte]) =
    digest(algorithm, data).map(x => "%02x".format(x)).reduce(_ + _)

  def md5_hex(data: Seq[Byte]) = hexdigest("MD5", data)
  def sha1_hex(data: Seq[Byte]) = hexdigest("SHA1", data)

  def md5(data: Seq[Byte]) = digest("MD5", data)
  def sha1(data: Seq[Byte]) = digest("SHA1", data)

  private def makeUUID(data: Seq[Byte], version: Int) = {
    val newData    = data.updated(6, ((data(6) & 0x0F) | (version << 4)).toByte)
                         .updated(8, ((data(8) & 0x3F) | 0x80).toByte)
    val buffer = ByteBuffer.wrap(newData.toArray).asLongBuffer()
    new UUID(buffer.get(0), buffer.get(1))
  }
  def uuidToBytes(namespace: UUID) =
    ByteBuffer.allocate(16).putLong(namespace.getMostSignificantBits).putLong(namespace.getLeastSignificantBits).array
  def md5_uuid (namespace: UUID, data: Seq[Byte]) = makeUUID(md5 (uuidToBytes(namespace) ++ data), 3)
  def sha1_uuid(namespace: UUID, data: Seq[Byte]) = makeUUID(sha1(uuidToBytes(namespace) ++ data), 5)
}